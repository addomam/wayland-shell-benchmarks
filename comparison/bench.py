#!/usr/bin/env python3
"""Compositor-observed comparison; no claim of photon or first-frame latency."""
import argparse
import copy
import json
import os
from pathlib import Path
import signal
import socket
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
SOCKET = Path(os.environ['XDG_RUNTIME_DIR']) / 'hypr' / os.environ['HYPRLAND_INSTANCE_SIGNATURE'] / '.socket.sock'


def ipc(command):
    with socket.socket(socket.AF_UNIX) as sock:
        sock.settimeout(3)
        sock.connect(str(SOCKET))
        sock.sendall(command.encode())
        parts = []
        while chunk := sock.recv(65536):
            parts.append(chunk)
    return b''.join(parts).decode()


def query(command):
    return json.loads(ipc('j/' + command))


def layers(prefix):
    result = []
    for output, data in query('layers').items():
        for level, entries in data['levels'].items():
            for entry in entries:
                if entry.get('namespace', '').startswith(prefix):
                    result.append(dict(entry, output=output, level=int(level)))
    return result


def write_json(path, data):
    pending = path.with_suffix('.next')
    pending.write_text(json.dumps(data))
    pending.replace(path)


class Runtime:
    def __init__(self, spec, destination):
        self.spec = spec
        self.destination = destination
        self.temp = tempfile.TemporaryDirectory(prefix='shell-compare-')
        self.config = Path(self.temp.name) / 'surfaces.json'
        self.control = Path(self.temp.name) / 'control'
        self.command_nonce = time.monotonic_ns() % 997
        self.layout = json.loads((ROOT / spec['config']).read_text())
        write_json(self.config, self.layout)
        self.control.write_text('')
        values = {'config': str(self.config), 'control': str(self.control), 'root': str(ROOT)}
        env = dict(os.environ, **{k: v.format(**values) for k, v in spec.get('env', {}).items()})
        self.log_path = destination.with_suffix('.log')
        self.log = self.log_path.open('w')
        self.start = time.perf_counter()
        self.process = subprocess.Popen([s.format(**values) for s in spec['command']],
                                        cwd=ROOT / spec['cwd'], env=env,
                                        stdout=self.log, stderr=subprocess.STDOUT,
                                        start_new_session=True)

    def wait(self, predicate, timeout=15):
        deadline = time.perf_counter() + timeout
        while time.perf_counter() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError(f"{self.spec['name']} exited {self.process.returncode}; see {self.log_path}")
            found = [entry for entry in layers(self.spec['namespace']) if entry.get('alpha', 1) > 0]
            if predicate(found):
                return found
            time.sleep(.01)
        raise TimeoutError(f"{self.spec['name']} layer condition timed out; layers={found}; log={self.log_path}")

    def bars(self):
        prefix = self.spec['namespace'] + 'bar@'
        count = len(query('monitors'))
        return self.wait(lambda items: len([x for x in items if x['namespace'].startswith(prefix) and x['h'] == 44]) == count)

    def command(self, text):
        # Consecutive commands must differ even when their text is identical.
        self.command_nonce = (self.command_nonce + 1) % 997
        self.control.write_text(text + '\n' + ' ' * (self.command_nonce + 1))

    def close(self):
        try:
            self.command('quit')
            self.process.wait(timeout=4)
        except (OSError, subprocess.TimeoutExpired):
            pass
        try:
            os.killpg(self.process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(self.process.pid, signal.SIGKILL)
            self.process.wait()
        self.log.close()
        self.temp.cleanup()


def process_tree(pid):
    processes = {}
    for path in Path('/proc').glob('[0-9]*/stat'):
        try:
            text = path.read_text()
            fields = text[text.rfind(')') + 2:].split()
            processes[int(path.parent.name)] = (int(fields[1]), int(fields[11]) + int(fields[12]))
        except (OSError, ValueError):
            continue
    selected = {pid}
    while True:
        fresh = selected | {p for p, (parent, _) in processes.items() if parent in selected}
        if selected == fresh:
            return {p: processes[p][1] for p in selected if p in processes}
        selected = fresh


def resource_sample(pid):
    ticks = process_tree(pid)
    rss = pss = 0
    denied = []
    stores = set()
    names = []
    for child in ticks:
        try:
            names.append(Path(f'/proc/{child}/comm').read_text().strip())
            for line in Path(f'/proc/{child}/smaps_rollup').read_text().splitlines():
                if line.startswith('Rss:'):
                    rss += int(line.split()[1])
                if line.startswith('Pss:'):
                    pss += int(line.split()[1])
            for line in Path(f'/proc/{child}/maps').read_text().splitlines():
                if '/nix/store/' in line:
                    path = line.split('/nix/store/', 1)[1].split('/')[0]
                    stores.add('/nix/store/' + path)
        except OSError as error:
            denied.append(f'{child}: {error}')
    return dict(rss_kib=rss, pss_kib=pss, processes=len(ticks), names=names,
                ticks=ticks, store_paths=sorted(stores), errors=denied)


def resources(runtime, duration):
    samples = []
    seen_ticks = {}
    cpu_ticks = 0
    system_before = [int(x) for x in Path('/proc/stat').read_text().splitlines()[0].split()[1:9]]
    start = time.perf_counter()
    while time.perf_counter() - start < duration:
        sample = resource_sample(runtime.process.pid)
        for pid, tick in sample.pop('ticks').items():
            if pid in seen_ticks:
                cpu_ticks += max(0, tick - seen_ticks[pid])
            seen_ticks[pid] = tick
        samples.append(sample)
        time.sleep(.25)
    elapsed = time.perf_counter() - start
    system_after = [int(x) for x in Path('/proc/stat').read_text().splitlines()[0].split()[1:9]]
    delta = [b - a for a, b in zip(system_before, system_after)]
    return dict(duration_s=elapsed, cpu_percent_one_core=cpu_ticks / os.sysconf('SC_CLK_TCK') / elapsed * 100,
                host_busy_percent=100 * (1 - (delta[3] + delta[4]) / max(1, sum(delta))),
                median_pss_mib=statistics.median(s['pss_kib'] for s in samples) / 1024,
                median_rss_mib=statistics.median(s['rss_kib'] for s in samples) / 1024,
                samples=samples)


def measure(spec, destination, idle_seconds):
    runtime = Runtime(spec, destination)
    data = {'name': spec['name'], 'command': spec['command']}
    try:
        data['initial_layers'] = runtime.bars()
        data['startup_layer_ms'] = (time.perf_counter() - runtime.start) * 1000
        time.sleep(2)
        data['idle'] = resources(runtime, idle_seconds)
        start = time.perf_counter()
        runtime.command('toggle launcher')
        prefix = spec['namespace'] + 'launcher@'
        runtime.wait(lambda items: any(x['namespace'].startswith(prefix) and x['w'] == 560 and x['h'] == 390 for x in items))
        data['launcher_control_layer_ms'] = (time.perf_counter() - start) * 1000
        runtime.command('toggle launcher')
        runtime.wait(lambda items: not any(x['namespace'].startswith(prefix) for x in items))
        visible = copy.deepcopy(runtime.layout)
        for surface in visible['surfaces']:
            surface['visible'] = True
        write_json(runtime.config, visible)
        runtime.wait(lambda items: len(items) == len(visible['surfaces']) * len(query('monitors')))
        time.sleep(1)
        data['active'] = resources(runtime, idle_seconds)
    finally:
        runtime.close()
    return data


def functional(spec, destination):
    base_reservations = {m['name']: m['reserved'] for m in query('monitors')}
    runtime = Runtime(spec, destination)
    checks = []
    try:
        items = runtime.bars()
        checks.append({'check': 'bars_on_all_outputs', 'layers': items})
        original = copy.deepcopy(runtime.layout)
        bar = next(s for s in runtime.layout['surfaces'] if s['id'] == 'bar')
        bar['height'] = 56
        bar['exclusiveZone'] = 56
        write_json(runtime.config, runtime.layout)
        items = runtime.wait(lambda xs: all(x['h'] == 56 for x in xs if 'bar@' in x['namespace']) and len(xs) > 0)
        time.sleep(.2)
        assert all(m['reserved'][1] == base_reservations[m['name']][1] + 56 for m in query('monitors')), 'exclusive zone mismatch'
        checks.append({'check': 'height_zone', 'layers': items, 'monitors': query('monitors')})
        runtime.config.write_text('{')
        time.sleep(.8)
        assert all(x['h'] == 56 for x in layers(spec['namespace']) if 'bar@' in x['namespace'])
        checks.append({'check': 'malformed_retains_layout', 'passed': True})
        bar['layer'] = 'bottom'
        bar['anchors'] = ['bottom', 'left', 'right']
        write_json(runtime.config, runtime.layout)
        items = runtime.wait(lambda xs: all(x['level'] == 1 for x in xs if 'bar@' in x['namespace']) and len(xs) > 0)
        checks.append({'check': 'move_bottom', 'layers': items})
        menu = copy.deepcopy(next(s for s in original['surfaces'] if s['id'] == 'menu'))
        menu.update(id='notes', visible=True)
        runtime.layout['surfaces'].append(menu)
        write_json(runtime.config, runtime.layout)
        items = runtime.wait(lambda xs: any('notes@' in x['namespace'] for x in xs))
        checks.append({'check': 'add_surface', 'layers': items})
        menu['page'] = next(s['page'] for s in original['surfaces'] if s['id'] == 'launcher')
        write_json(runtime.config, runtime.layout)
        time.sleep(.8)
        checks.append({'check': 'page_change_host_alive', 'passed': runtime.process.poll() is None})
        write_json(runtime.config, original)
        runtime.wait(lambda xs: not any('notes@' in x['namespace'] for x in xs) and all(x['h'] == 44 for x in xs if 'bar@' in x['namespace']))
        checks.append({'check': 'remove_restore', 'passed': True})
        for name in ['launcher', 'menu', 'input-test']:
            runtime.command('toggle ' + name)
            expected = next(s for s in original['surfaces'] if s['id'] == name)
            opened = runtime.wait(lambda xs: any(name + '@' in x['namespace'] and x['w'] == expected['width'] and x['h'] == expected['height'] for x in xs))
            checks.append({'check': 'show_' + name, 'layers': opened})
            runtime.command('toggle ' + name)
            runtime.wait(lambda xs: not any(name + '@' in x['namespace'] for x in xs))
        runtime.command('reload')
        time.sleep(.5)
        checks.append({'check': 'reload_host_alive', 'passed': runtime.process.poll() is None})
        return {'name': spec['name'], 'checks': checks, 'baseline_reservations': base_reservations}
    finally:
        runtime.close()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--manifest', default=str(ROOT / 'comparison/implementations.json'))
    parser.add_argument('--only', default='')
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--sample-seconds', type=float, default=5)
    parser.add_argument('--functional', action='store_true')
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    specs = json.loads(Path(args.manifest).read_text())
    if args.only:
        specs = [s for s in specs if s['name'] in args.only.split(',')]
    output = Path(args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    results = {'monitors': query('monitors'), 'hyprland': query('version'), 'samples': [], 'errors': [],
               'method': 'direct-process startup to bar layer entries with configured height and nonzero compositor alpha; launcher to configured geometry/nonzero alpha; no guarantee of content frame/presentation; 10ms polling; warm OS cache; CPU percent of one core; summed child-process RSS and PSS'}
    for iteration in range(1 if args.functional else args.runs):
        # Rotate launch order so one implementation does not always get the same place.
        order = specs[iteration % len(specs):] + specs[:iteration % len(specs)]
        for spec in order:
            print(f"{spec['name']} run {iteration + 1}", flush=True)
            destination = output.parent / (output.stem + f"-{spec['name']}-{iteration + 1}")
            try:
                result = functional(spec, destination) if args.functional else measure(spec, destination, args.sample_seconds)
                result['iteration'] = iteration + 1
                results['samples'].append(result)
            except Exception as error:
                results['errors'].append({'name': spec['name'], 'iteration': iteration + 1, 'error': str(error)})
                print(f'FAILED: {error}', flush=True)
            output.write_text(json.dumps(results, indent=2))
            time.sleep(.5)
    if results['errors']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()

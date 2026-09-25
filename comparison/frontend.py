#!/usr/bin/env python3
"""Exercise renderer-specific content edits independently of topology."""
import json
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.request
from bench import ROOT, Runtime, layers
from interactions import screenshot


def wait_log(runtime, marker, start=0):
    deadline = time.perf_counter() + 8
    while time.perf_counter() < deadline:
        if marker in runtime.log_path.read_text()[start:]:
            return
        if runtime.process.poll() is not None:
            raise RuntimeError('runtime exited during frontend reload')
        time.sleep(.05)
    raise TimeoutError('missing frontend marker: ' + marker)


def local(spec):
    name = spec['name']
    asset = ROOT / spec['cwd'] / {'web': 'assets/style.css', 'iced': 'pages/bar.json', 'quickshell': 'pages/Bar.qml'}[name]
    original = asset.read_text()
    destination = ROOT / 'comparison/results' / ('frontend-' + name)
    runtime = Runtime(spec, destination)
    try:
        runtime.bars()
        time.sleep(1)
        before = {x['namespace']: x['address'] for x in layers(spec['namespace'])}
        log_start = len(runtime.log_path.read_text())
        if name == 'web':
            changed = original + '\n.bar { background: #304050; }\n'
            marker = 'ASSET_RELOAD'
        elif name == 'iced':
            page = json.loads(original)
            page['background'] = '#304050'
            changed = json.dumps(page)
            marker = 'CONTENT'
        else:
            changed = original.replace('#202332', '#304050')
            marker = 'Reloading configuration'
        asset.write_text(changed)
        wait_log(runtime, marker, log_start)
        time.sleep(.6)
        after = {x['namespace']: x['address'] for x in runtime.bars()}
        bar = next(x for x in layers(spec['namespace']) if 'bar@' in x['namespace'])
        screenshot(bar, destination.with_suffix('.png'))
        return {'name': name, 'host_pid_retained': runtime.process.poll() is None,
                'native_addresses_before': before, 'native_addresses_after': after,
                'native_addresses_retained': before == after, 'marker': marker}
    finally:
        asset.write_text(original)
        time.sleep(.5)
        runtime.close()


def vite(spec):
    project = ROOT / 'webshell-prototype'
    asset = project / 'assets/style.css'
    original = asset.read_text()
    server_log = (ROOT / 'comparison/results/vite.log').open('w')
    server = subprocess.Popen(['nix', 'develop', 'path:.', '-c', 'npm', 'run', 'dev'], cwd=project,
                              stdout=server_log, stderr=subprocess.STDOUT, start_new_session=True)
    runtime = None
    try:
        deadline = time.perf_counter() + 25
        while True:
            try:
                urllib.request.urlopen('http://127.0.0.1:5173/dev-bar.html', timeout=1).close()
                break
            except OSError:
                if time.perf_counter() > deadline:
                    raise TimeoutError('Vite server did not start')
                time.sleep(.2)
        with tempfile.TemporaryDirectory(prefix='shell-vite-') as temp:
            config = Path(temp) / 'surfaces.json'
            layout = json.loads((ROOT / spec['config']).read_text())
            next(s for s in layout['surfaces'] if s['id'] == 'bar')['page'] = 'http://127.0.0.1:5173/dev-bar.html'
            config.write_text(json.dumps(layout))
            adapted = dict(spec, config=str(config))
            runtime = Runtime(adapted, ROOT / 'comparison/results/frontend-vite')
            runtime.bars()
            wait_log(runtime, 'css:rgb(32, 35, 50)')
            before = {x['namespace']: x['address'] for x in layers(spec['namespace'])}
            asset.write_text(original + '\n.bar { background: #304050; }\n')
            wait_log(runtime, 'css:rgb(48, 64, 80)')
            after = {x['namespace']: x['address'] for x in layers(spec['namespace'])}
            return {'name': 'web-vite', 'computed_css_changed': True, 'native_addresses_retained': before == after}
    finally:
        asset.write_text(original)
        if runtime:
            runtime.close()
        try:
            subprocess.run(['kill', '-TERM', '--', '-' + str(server.pid)], capture_output=True)
            server.wait(timeout=4)
        except (ProcessLookupError, subprocess.TimeoutExpired):
            pass
        server_log.close()


def main():
    specs = json.loads((ROOT / 'comparison/implementations.json').read_text())
    result = {'checks': [], 'errors': []}
    for spec in specs:
        print('frontend', spec['name'], flush=True)
        try:
            result['checks'].append(local(spec))
        except Exception as error:
            result['errors'].append({'name': spec['name'], 'error': str(error)})
    try:
        print('frontend Vite HMR', flush=True)
        result['checks'].append(vite(next(s for s in specs if s['name'] == 'web')))
    except Exception as error:
        result['errors'].append({'name': 'web-vite', 'error': str(error)})
    (ROOT / 'comparison/results/frontend.json').write_text(json.dumps(result, indent=2))
    if result['errors']:
        print(result['errors'], flush=True)
        raise SystemExit(1)


if __name__ == '__main__':
    main()

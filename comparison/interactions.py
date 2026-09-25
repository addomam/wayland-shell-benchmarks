#!/usr/bin/env python3
"""Actual virtual-pointer clicks plus IPC workspace observations; restores cursor/workspace."""
import argparse
import copy
import json
import subprocess
import time
from bench import ROOT, Runtime, ipc, query, layers, write_json

WLRCTL = 'wlrctl'
GRIM = 'grim'


def move(x, y):
    cursor = query('cursorpos')
    subprocess.run([WLRCTL, 'pointer', 'move', str(round(x - cursor['x'])), str(round(y - cursor['y']))], check=True, capture_output=True)
    time.sleep(.06)


def click():
    subprocess.run([WLRCTL, 'pointer', 'click', 'left'], check=True, capture_output=True)


def point_bar(namespace):
    # Monitor focus changes can alter the compositor's usable-area placement.
    for _ in range(2):
        bar = next(x for x in layers(namespace) if x['namespace'] == namespace)
        move(bar['x'] + 38, bar['y'] + 22)
    return bar


def screenshot(layer, filename):
    subprocess.run([GRIM, '-g', f"{layer['x']},{layer['y']} {layer['w']}x{layer['h']}", str(filename)], check=True, capture_output=True)


def wait_workspace(id, timeout=4):
    deadline = time.perf_counter() + timeout
    while time.perf_counter() < deadline:
        if query('activeworkspace')['id'] == id:
            return
        time.sleep(.01)
    raise TimeoutError('workspace switch not observed')


def restore_workspaces(monitors, focused_workspace):
    for monitor in monitors:
        ipc('dispatch hl.dsp.focus({ monitor = ' + json.dumps(monitor['name']) + ' })')
        ipc('dispatch hl.dsp.focus({ workspace = "' + str(monitor['activeWorkspace']['id']) + '", on_current_monitor = true })')
    ipc(f'dispatch hl.dsp.focus({{ workspace = "{focused_workspace}" }})')


def test(spec, destination):
    before_cursor = query('cursorpos')
    before_workspace = query('activeworkspace')['id']
    before_monitors = query('monitors')
    runtime = Runtime(spec, destination)
    result = {'name': spec['name'], 'pointer_launcher_ms': [], 'input_regions': []}
    try:
        bars = runtime.bars()
        time.sleep(2)
        bars = [x for x in bars if 'bar@' in x['namespace']]
        for bar in bars:
            launcher = spec['namespace'] + 'launcher@' + bar['output']
            for _ in range(3):
                bar = point_bar(bar['namespace'])
                start = time.perf_counter()
                click()
                shown = runtime.wait(lambda xs: any(x['namespace'] == launcher and x['w'] == 560 and x['h'] == 390 for x in xs))
                result['pointer_launcher_ms'].append({'output': bar['output'], 'ms': (time.perf_counter() - start) * 1000})
                time.sleep(.4)
                if _ == 0:
                    screenshot(next(x for x in shown if x['namespace'] == launcher), destination.with_name(destination.name + '-launcher-' + bar['output']).with_suffix('.png'))
                window = next(x for x in shown if x['namespace'] == launcher)
                move(window['x'] + window['w'] - 70, window['y'] + 45)
                click()
                runtime.wait(lambda xs: not any(x['namespace'] == launcher for x in xs))
        test_workspace = next(i for i in range(99, 80, -1) if i not in [w['id'] for w in query('workspaces')])
        start = time.perf_counter()
        runtime.command('workspace ' + str(test_workspace))
        wait_workspace(test_workspace)
        result['workspace_control_ipc_ms'] = (time.perf_counter() - start) * 1000
        time.sleep(.7)
        result['workspace_log_evidence'] = [line for line in runtime.log_path.read_text().splitlines() if 'WORKSPACES' in line or 'active:' in line][-12:]
        current_bar = next(x for x in layers(spec['namespace']) if x['namespace'] == bars[-1]['namespace'])
        screenshot(current_bar, destination.with_name(destination.name + '-workspace').with_suffix('.png'))
        restore_workspaces(before_monitors, before_workspace)
        wait_workspace(before_workspace)
        for bar in bars:
            layout = copy.deepcopy(runtime.layout)
            overlay = next(s for s in layout['surfaces'] if s['id'] == 'input-test')
            overlay.update(width=0, height=360, anchors=['top', 'left', 'right'], margins={'top': -44}, output=bar['output'], visible=True)
            write_json(runtime.config, layout)
            key = spec['namespace'] + 'input-test@' + bar['output']
            shown = runtime.wait(lambda xs: any(x['namespace'] == key and x['w'] == bar['w'] for x in xs))
            region = next(x for x in shown if x['namespace'] == key)
            time.sleep(.5)
            card = dict(region, x=region['x'] + (region['w'] - 250) // 2,
                        y=region['y'] + 100, w=250, h=160)
            screenshot(card, destination.with_name(destination.name + '-input-' + bar['output']).with_suffix('.png'))
            bar = point_bar(bar['namespace'])
            click()
            launcher = spec['namespace'] + 'launcher@' + bar['output']
            launched = runtime.wait(lambda xs: any(x['namespace'] == launcher for x in xs))
            time.sleep(.4)
            window = next(x for x in launched if x['namespace'] == launcher)
            move(window['x'] + window['w'] - 70, window['y'] + 45)
            click()
            runtime.wait(lambda xs: not any(x['namespace'] == launcher for x in xs))
            region = next(x for x in layers(spec['namespace']) if x['namespace'] == key)
            offset = {'web': (0, 0), 'iced': (-70, -10), 'quickshell': (-65, 20)}[spec['name']]
            move(region['x'] + region['w'] / 2 + offset[0], region['y'] + 180 + offset[1])
            click()
            if spec['name'] == 'web':
                deadline = time.perf_counter() + 3
                while 'input-island-clicked' not in runtime.log_path.read_text() and time.perf_counter() < deadline:
                    time.sleep(.05)
                assert 'input-island-clicked' in runtime.log_path.read_text(), 'DOM input click not received'
            else:
                runtime.wait(lambda xs: not any(x['namespace'] == key for x in xs))
            result['input_regions'].append({'output': bar['output'], 'transparent_pass_through': True, 'interactive_received': True})
            write_json(runtime.config, runtime.layout)
            runtime.wait(lambda xs: not any('input-test@' in x['namespace'] for x in xs))
        return result
    finally:
        runtime.close()
        restore_workspaces(before_monitors, before_workspace)
        move(before_cursor['x'], before_cursor['y'])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--only', default='')
    args = parser.parse_args()
    specs = json.loads((ROOT / 'comparison/implementations.json').read_text())
    if args.only:
        specs = [s for s in specs if s['name'] in args.only.split(',')]
    result = {'method': 'virtual-pointer command start to correctly sized launcher layer with nonzero alpha; includes wlrctl invocation; not click-to-photon', 'samples': [], 'errors': []}
    for spec in specs:
        print('interactive test', spec['name'], flush=True)
        destination = ROOT / 'comparison/results' / ('interactions-' + spec['name'])
        try:
            result['samples'].append(test(spec, destination))
        except Exception as error:
            result['errors'].append({'name': spec['name'], 'error': str(error)})
            print('FAILED', error, flush=True)
        (ROOT / 'comparison/results' / ('interactions' + ('-' + args.only if args.only else '') + '.json')).write_text(json.dumps(result, indent=2))
    if result['errors']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()

#!/usr/bin/env python3
"""Launcher filtering/Escape and a real workspace-button action."""
import argparse
import json
import subprocess
import time
from bench import ROOT, Runtime, query, layers
from interactions import WLRCTL, move, click, point_bar, screenshot, wait_workspace, restore_workspaces


def test(spec):
    original_monitors = query('monitors')
    original_workspace = query('activeworkspace')['id']
    original_cursor = query('cursorpos')
    destination = ROOT / 'comparison/results' / ('keyboard-' + spec['name'])
    runtime = Runtime(spec, destination)
    result = {'name': spec['name']}
    try:
        runtime.bars()
        time.sleep(1)
        runtime.command('toggle launcher')
        shown = runtime.wait(lambda xs: any('launcher@' in x['namespace'] and x['w'] == 560 for x in xs))
        launcher = next(x for x in shown if 'launcher@' in x['namespace'])
        time.sleep(.5)
        for _ in range(2):
            launcher = next(x for x in layers(spec['namespace']) if x['namespace'] == launcher['namespace'])
            move(launcher['x'] + 110, launcher['y'] + (130 if spec['name'] == 'web' else 90))
        click()
        assert any(x['namespace'] == launcher['namespace'] for x in layers(spec['namespace'])), 'launcher closed before typing'
        subprocess.run([WLRCTL, 'keyboard', 'type', 'Files'], check=True, capture_output=True)
        time.sleep(.3)
        launcher = next(x for x in layers(spec['namespace']) if x['namespace'] == launcher['namespace'])
        screenshot(launcher, destination.with_suffix('.png'))
        result['filter_screenshot'] = str(destination.with_suffix('.png').relative_to(ROOT))
        subprocess.run([WLRCTL, 'keyboard', 'type', '\x1b'], check=True, capture_output=True)
        runtime.wait(lambda xs: not any('launcher@' in x['namespace'] for x in xs))
        result['escape_closes'] = True
        monitor = launcher['output']
        target = min(w['id'] for w in query('workspaces') if w['id'] > 0)
        fresh = next(i for i in range(99, 80, -1) if i not in [w['id'] for w in query('workspaces')])
        bar = point_bar(spec['namespace'] + 'bar@' + monitor)
        runtime.command('workspace ' + str(fresh))
        wait_workspace(fresh)
        time.sleep(.5)
        bar = next(x for x in layers(spec['namespace']) if x['namespace'] == bar['namespace'])
        screenshot(dict(bar, w=400), destination.with_name(destination.name + '-bar').with_suffix('.png'))
        offsets = {'web': [245, 250], 'iced': [160], 'quickshell': [185]}[spec['name']]
        for offset in offsets:
            move(bar['x'] + offset, bar['y'] + 22)
            if query('activeworkspace')['id'] != fresh:
                raise RuntimeError('pointer focus changed workspace before test click')
            start = time.perf_counter()
            click()
            deadline = time.perf_counter() + .5
            while time.perf_counter() < deadline:
                if query('activeworkspace')['id'] == target:
                    result['workspace_pointer_ipc_ms'] = (time.perf_counter() - start) * 1000
                    result['workspace_click_offset'] = offset
                    if spec['name'] == 'web':
                        log_start = len(runtime.log_path.read_text())
                        recovery_start = time.perf_counter()
                        runtime.command('crash bar')
                        recovery_deadline = time.perf_counter() + 8
                        while 'RECOVERED' not in runtime.log_path.read_text()[log_start:]:
                            if time.perf_counter() > recovery_deadline:
                                raise TimeoutError('WebKit view did not recover')
                            time.sleep(.02)
                        runtime.bars()
                        result['webview_recovery_layer_ms'] = (time.perf_counter() - recovery_start) * 1000
                        result['webview_crash_host_survived'] = runtime.process.poll() is None
                    return result
                time.sleep(.01)
        raise RuntimeError('workspace button was not activated')
    finally:
        runtime.close()
        restore_workspaces(original_monitors, original_workspace)
        move(original_cursor['x'], original_cursor['y'])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--only')
    args = parser.parse_args()
    output = ROOT / 'comparison/results/keyboard.json'
    result = json.loads(output.read_text()) if args.only and output.exists() else {'samples': [], 'errors': []}
    if args.only:
        result = {key: [s for s in values if s['name'] != args.only] for key, values in result.items()}
    for spec in json.loads((ROOT / 'comparison/implementations.json').read_text()):
        if args.only and spec['name'] != args.only:
            continue
        print('keyboard', spec['name'], flush=True)
        try:
            result['samples'].append(test(spec))
        except Exception as error:
            result['errors'].append({'name': spec['name'], 'error': str(error)})
            print('FAILED', error, flush=True)
    output.write_text(json.dumps(result, indent=2))
    if result['errors']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()

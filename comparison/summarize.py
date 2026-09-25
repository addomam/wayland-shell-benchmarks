#!/usr/bin/env python3
import json
import math
from pathlib import Path
import statistics

BASE = Path(__file__).resolve().parent


def distribution(values):
    ordered = sorted(values)
    return {'n': len(values), 'median': statistics.median(values),
            'p95_nearest_rank': ordered[math.ceil(.95 * len(ordered)) - 1],
            'min': ordered[0], 'max': ordered[-1]}


def main():
    measured = json.loads((BASE / 'results/performance.json').read_text())
    interactions = json.loads((BASE / 'results/interactions.json').read_text())
    summary = {}
    for name in ['web', 'iced', 'quickshell']:
        samples = [s for s in measured['samples'] if s['name'] == name]
        data = {}
        for metric in ['startup_layer_ms', 'launcher_control_layer_ms']:
            data[metric] = distribution([s[metric] for s in samples])
        for phase in ['idle', 'active']:
            for metric in ['median_pss_mib', 'median_rss_mib', 'cpu_percent_one_core', 'host_busy_percent']:
                data[phase + '_' + metric] = distribution([s[phase][metric] for s in samples])
            data[phase + '_processes'] = distribution([s['processes'] for sample in samples for s in sample[phase]['samples']])
            data[phase + '_memory_errors'] = sorted(set(error for sample in samples for s in sample[phase]['samples'] for error in s['errors']))
        pointer = next(s for s in interactions['samples'] if s['name'] == name)
        data['pointer_launcher_ms'] = distribution([s['ms'] for s in pointer['pointer_launcher_ms']])
        data['workspace_control_ipc_ms_single_sample'] = pointer['workspace_control_ipc_ms']
        summary[name] = data
    (BASE / 'results/summary.json').write_text(json.dumps(summary, indent=2))
    lines = ['# Measured summary', '',
             'Warm OS caches, release Rust builds, five launches per implementation in rotating order. Median / nearest-rank p95 in milliseconds; with five samples p95 is the maximum. These are compositor observation boundaries, not photon or content-ready times.', '',
             '| Implementation | Bar startup ms | Launcher control ms | Launcher pointer ms |',
             '| --- | ---: | ---: | ---: |']
    for name, data in summary.items():
        values = [data[k] for k in ['startup_layer_ms', 'launcher_control_layer_ms', 'pointer_launcher_ms']]
        lines.append('| ' + name + ' | ' + ' | '.join(f"{v['median']:.1f} / {v['p95_nearest_rank']:.1f}" for v in values) + ' |')
    lines += ['', 'Historical five-run results; see CPU_ANALYSIS.md for the subsequent optimization and measurements. Pointer timings use six clicks (three on each output) and include virtual-pointer tool invocation. Control-file timings include the 150 ms polling design.', '',
              '| Implementation | Idle PSS MiB | Active PSS MiB | Idle CPU % one core | Active CPU % one core | Idle / active processes |',
              '| --- | ---: | ---: | ---: | ---: | ---: |']
    for name, d in summary.items():
        lines.append(f"| {name} | {d['idle_median_pss_mib']['median']:.1f} | {d['active_median_pss_mib']['median']:.1f} | {d['idle_cpu_percent_one_core']['median']:.2f} | {d['active_cpu_percent_one_core']['median']:.2f} | {d['idle_processes']['median']:g} / {d['active_processes']['median']:g} |")
    lines += ['', 'PSS sums the application process tree, including WebKit children. Active shows all eight surfaces; idle shows two bars. Each sample window is five seconds. GPU allocations and compositor cost are not included. Raw RSS, samples, host load, commands, logs, and errors are retained in results/.', '']
    (BASE / 'SUMMARY.md').write_text('\n'.join(lines))
    print('\n'.join(lines))


if __name__ == '__main__':
    main()

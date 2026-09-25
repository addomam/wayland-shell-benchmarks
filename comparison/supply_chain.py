#!/usr/bin/env python3
"""Lockfile inventory and OSV evidence; native Nix libraries need separate review."""
import argparse
import datetime
import json
from pathlib import Path
import subprocess
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parent.parent


def inventory(project):
    path = ROOT / project
    data = {'project': project, 'packages': [], 'source_lines': {}, 'unsafe_lines': []}
    lock = path / 'Cargo.lock'
    if lock.exists():
        cargo = tomllib.loads(lock.read_text())
        for package in cargo['package']:
            if 'source' in package:
                data['packages'].append({'ecosystem': 'crates.io', 'name': package['name'],
                                         'version': package['version'], 'source': package['source'],
                                         'checksum': package.get('checksum')})
        data['cargo_direct'] = tomllib.loads((path / 'Cargo.toml').read_text()).get('dependencies', {})
    npm = path / 'package-lock.json'
    if npm.exists():
        for key, package in json.loads(npm.read_text())['packages'].items():
            if key:
                data['packages'].append({'ecosystem': 'npm', 'name': key.rsplit('node_modules/', 1)[-1],
                                         'version': package['version'], 'source': package.get('resolved'),
                                         'dev': package.get('dev', False), 'integrity': package.get('integrity')})
    for source in sorted(path.rglob('*')):
        if any(part in {'target', 'node_modules', '.git'} for part in source.relative_to(path).parts):
            continue
        if source.is_file() and source.suffix in {'.rs', '.qml', '.js', '.html', '.css'}:
            lines = source.read_text().splitlines()
            data['source_lines'][str(source.relative_to(path))] = len(lines)
            data['unsafe_lines'].extend(f'{source.relative_to(path)}:{i}' for i, line in enumerate(lines, 1) if 'unsafe ' in line)
    return data


def osv(packages):
    results = []
    for offset in range(0, len(packages), 100):
        batch = packages[offset:offset + 100]
        queries = [{'package': {'ecosystem': p['ecosystem'], 'name': p['name']}, 'version': p['version']} for p in batch]
        payload = json.dumps({'queries': queries}).encode()
        request = urllib.request.Request('https://api.osv.dev/v1/querybatch', data=payload,
                                         headers={'Content-Type': 'application/json'})
        response = json.load(urllib.request.urlopen(request, timeout=30))
        for package, answer in zip(batch, response['results']):
            if answer.get('vulns') or answer.get('next_page_token'):
                results.append({'package': package, 'response': answer})
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--osv', action='store_true')
    parser.add_argument('--measurements')
    args = parser.parse_args()
    prior_path = ROOT / 'comparison/results/supply-chain.json'
    prior = json.loads(prior_path.read_text()) if prior_path.exists() else {'projects': []}
    data = {'date': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'projects': [],
            'scope': 'Cargo/npm lock versions, including optional target packages. OSV does not audit native GTK/WebKit/Qt/Mesa/Nix packages or prove exploitability. Closure inventory reflects measured mapped store paths plus their Nix references.'}
    for name in ['webshell-prototype', 'iced-shell', 'quickshell-shell']:
        project = inventory(name)
        if args.osv:
            try:
                project['osv'] = osv(project['packages'])
            except Exception as error:
                project['osv_error'] = str(error)
        else:
            old = next((p for p in prior['projects'] if p['project'] == name), {})
            for key in ['osv', 'osv_error']:
                if key in old:
                    project[key] = old[key]
            project['osv_scan_date'] = prior.get('date')
        data['projects'].append(project)
        print(name, len(project['packages']), 'locked packages', flush=True)
    if args.measurements:
        measurements = json.loads(Path(args.measurements).read_text())
        for short, project in zip(['web', 'iced', 'quickshell'], data['projects']):
            roots = set()
            for sample in measurements['samples']:
                if sample['name'] == short:
                    for phase in ['idle', 'active']:
                        for memory in sample[phase]['samples']:
                            roots.update(memory['store_paths'])
            project['mapped_store_roots'] = sorted(roots)
            excluded = {p for p in roots if p.endswith(('-system-path', '-fc-cache'))}
            project['excluded_environment_aggregators'] = sorted(excluded)
            roots -= excluded
            if roots:
                try:
                    result = subprocess.run(['nix', 'path-info', '--json', '--recursive', *sorted(roots)], text=True, capture_output=True, check=True)
                    closure = json.loads(result.stdout)
                    if isinstance(closure, dict):
                        paths = list(closure.values())
                    else:
                        paths = closure
                    project['closure_paths'] = len(paths)
                    project['closure_nar_bytes'] = sum(p['narSize'] for p in paths)
                    (ROOT / 'comparison/results' / f'{short}-closure.json').write_text(json.dumps(closure, indent=2))
                except Exception as error:
                    project['closure_error'] = str(error)
    (ROOT / 'comparison/results/supply-chain.json').write_text(json.dumps(data, indent=2))


if __name__ == '__main__':
    main()

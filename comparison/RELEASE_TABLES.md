# Release validation measurements

Medians; CPU is percent of one core, memory is summed process-tree MiB. Startup ends at compositor-observed layer mapping, not first painted frame.

| Implementation | n | Startup ms | Idle PSS | Active PSS | Idle CPU % | Active CPU % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| iced | 10 | 126.808 | 42.405 | 43.074 | 0.333 | 0.333 |
| quickshell | 10 | 168.225 | 94.356 | 122.190 | 0.959 | 1.495 |
| web | 10 | 407.516 | 441.208 | 511.205 | 3.393 | 5.513 |

| Implementation | Idle / active RSS MiB | Processes idle / active | Startup p95 ms | Control launcher median / p95 ms | Host busy idle / active % |
| --- | ---: | ---: | ---: | ---: | ---: |
| iced | 69.9 / 70.6 | 1 / 1 | 138.3 | 32.4 / 34.1 | 5.29 / 5.22 |
| quickshell | 131.4 / 160.9 | 1 / 1 | 186.6 | 112.4 / 202.4 | 5.34 / 5.63 |
| web | 1701.0 / 1821.2 | 50 / 50 | 424.8 | 44.5 / 85.3 | 6.85 / 7.14 |

Recorded failures: 0. Inspect the input JSON for all raw samples and errors.

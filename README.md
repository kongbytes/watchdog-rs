# Network watchdog

[![dependency status](https://deps.rs/repo/github/kongbytes/watchdog-rs/status.svg)](https://deps.rs/repo/github/kongbytes/watchdog-rs)


A minimalist & multi-region network monitoring tool written in Rust. Monitor network failures with custom tests and multiple alerting modes (Telegram, SMS, ...)

## Project goals

- **All-in-one solution** : the project can be used to push metrics from network regions, aggregate all metrics in a server and see the results with a CLI
- **Minimal footprint** : the major open-source monitoring solutions such as Prometheus and Zabbix offer a wide range of features, but consume a lot of resources (memory, network, ...). This project provides a set of components that are designed for low-resource consumption.
- **Default "push" approach** : to enable multi-region network monitoring, the server defaults to a "push" approach. No need to have a fixed IP address or configure NAT in your router : all data is sent from the regions to the main public server.
- **Grafana-ready** : while the project provides a CLI to view the status of your regions and track incidents, the project also integrates with Grafana.
- **Configuration as code** : easily backup your configuration, keep track of changes with Git and share with other teams.


## Getting started

Download the `watchdog` binary for Linux (Ubuntu, Fedora, Debian, ...). See the [releases page](https://github.com/kongbytes/watchdog-rs/releases) for other binaries.

```bash
wget -O watchdog https://github.com/kongbytes/watchdog-rs/releases/download/v0.4.1/watchdog-rs-v0.4.1-x86_64-unknown-linux-musl && chmod +x ./watchdog
```

Create a YAML configuration `config.yaml` file as shown below. This **configuration file** will be used by the monitoring server to manage your configuration accross regions.

```yaml
regions:
  - name: local-network
    groups:
      - name: default
        tests:
          - http kongbytes.io
```

Launch the main **monitoring server** that will be used by network regions to collect metrics. This service should be reachable by all network regions on port `3030`.

```bash
# Define a set of environment variables that will be used by the server
export WATCHDOG_TOKEN=your_secret_token

# Launch the main watchdog server on port 3030
watchdog server --config ./config.yaml
```

In a region, launch a **network relay** : a service that will retrieve the monitoring configuration from the server and start performing network tests. Each time a test is performed, the results will be pushed to the main monitoring server.

```bash
# Define a set of environment variables that will be used
# by the relay. In a production environment, this would
# be defined in a systemd service.
export WATCHDOG_ADDR=http://localhost:3030
export WATCHDOG_TOKEN=your_secret_token

# Launch a watchdog network region relay
watchdog relay --region local-network
```

On your workstation, use the **CLI** to get details about the monitoring state & ongoing incidents.

```bash
# Put these environment variables in a safe place on your
# workstation (watch out for shell history)
WATCHDOG_ADDR=http://localhost:3030
WATCHDOG_TOKEN=your_secret_token

# Get the status of all your network regions & zones
watchdog status
```

## Roadmap

Docs
- Production setup

CLI
- Complete existing CLI commands
- Server management (alerters, external IP, ...)
- Bash autocompletion

Server
- Alerters (SMS, email, webhook, script, ...)
- Bandwidth control for relays

Relay
- ~~Auto-detect config changes~~
- Add commands (TCP, UDP, ARP, ...)
- Custom command (track UPS failure, ...)
- ~~Add metrics (HTTP latency, ...)~~

Docs & more
- Real-life scenarios (Raspberry, ...)

## Contributing

Feel free to suggest an improvement, report a bug, or ask something: https://github.com/kongbytes/watchdog-rs/issues

# Network watchdog

A minimalist & multi-region network monitoring tool written in Rust

## Getting started

Start by retrieving the latest release of the `watchdog-rs` project and create a YAML configuration `config.yaml` file as shown below. This **configuration file** will be used by the monitoring server to manage your configuration accross regions.

```yaml
regions:
  - name: eu-west
    interval: 5s # Interval between network checks in a region
    threshold: 3 # Amount of region failures tolerated before alert
    groups:
      - name: default
        threshold: 4 # Amount of zone failures tolerted before alert
        mediums: telegram # Alert mediums
        tests:
          - http www.lasemo.be
```

Launch the main **monitoring server** that will be used by network regions to collect metrics. This service should be reachable by all network regions on port `3030`.

```bash
# Define a set of environment variables that will be used
# by the server. In a production environment, this would
# be defined in a systemd service.
export TELEGRAM_TOKEN=x
export TELEGRAM_CHAT=x
export WATCHDOG_TOKEN=x

# Launch the main watchdog server on port 3030
watchdog server --config ./config.yaml
```

In a region, launch a **network relay** : a service that will retrieve the monitoring configuration from the server and start performing network tests. Each time a test is performed, the results will be pushed to the main monitoring server.

```bash
# Define a set of environment variables that will be used
# by the relay. In a production environment, this would
# be defined in a systemd service.
export WATCHDOG_ADDR=http://localhost:3030
export WATCHDOG_TOKEN=x

# Launch a watchdog network region relay
watchdog relay --region lasemo-qg
```

On your workstation, use the **CLI** to get details about the monitoring state & ongoing incidents.

```bash
# Put these environment variables in a safe place on your
# workstation (watch out for shell history)
WATCHDOG_ADDR=http://localhost:3030
WATCHDOG_TOKEN=x

# Get the status of all your network regions & zones
watchdog status
```

## Project goals

- **All-in-one solution** : the project can be used to push metrics from network regions, aggregate all metrics in a server and see the results with a CLI
- **Minimal footprint** : the major open-source monitoring solutions such as Prometheus and Zabbix offer a wide range of features, but consume a lot of resources (memory, network, ...). This project provides a set of components that are designed for low-resource consumption.
- **Default "push" approach** : to enable multi-region network monitoring, the server defaults to a "push" approach. No need to have a fixed IP address or configure NAT in your router : all data is sent from the regions to the main public server.
- **Grafana-ready** : while the project provides a CLI to view the status of your regions and track incidents, the project also integrates with Grafana.
- **Configuration as code** : easily backup your configuration, keep track of changes with Git and share with other teams.

## Roadmap

CLI
- Complete existing CLI commands
- Server management (alerters, external IP, ...)
- Bash autocompletion

Server
- Alerters (SMS, email, webhook, script, ...)
- Bandwidth control for relays

Relay
- Auto-detect config changes
- Add commands (TCP, UDP, ARP, ...)
- Add metrics (HTTP latency, ...)

Docs & more
- Real-life scenarios (Raspberry, ...)
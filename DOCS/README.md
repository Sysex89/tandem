# Documentation map (Diátaxis)

Project documentation is organized with [Diátaxis](https://diataxis.fr/): four kinds of doc match four reader needs. Service-specific notes live next to the code (`srcs/requirements/<service>/README.md`, and `frontend/docs/` for the frontend).

| Need | Where |
|------|--------|
| Learning: a guided path through a topic | [tutorial/](tutorial/) |
| Doing: complete a specific task | [howto/](howto/) |
| Looking up facts (paths, names, contracts) | [reference/](reference/) |
| Understanding why things are built this way | [explanation/](explanation/) |

## Tutorials

- [Editor service: real-time collaboration](tutorial/editor_service_ws.md): WebSockets in Axum, rooms, sync, persistence
- [Monitoring from zero](tutorial/monitoring-from-zero.md): first walk through Grafana and Prometheus

## How-to guides

- [Deploy the hosted demo](howto/deploy-the-demo.md)
- [Update the auth database schema](howto/auth-new-migration-howto.md)
- [Add a new application metric](howto/add-a-new-application-metric.md)
- [Verify Prometheus and Grafana metrics](howto/verify-prometheus-and-grafana-metrics.md)
- [Restore auth after new npm dependencies](howto/restore-auth-after-new-npm-dependencies.md) (`make dev`)

## Reference

- [Database schema](reference/database-schema.md)
- [Application and infrastructure metrics](reference/application-and-infrastructure-metrics.md)
- [Monitoring files and ports](reference/monitoring-files-and-ports.md)

## Explanation

- [Load test: how many people can edit one note?](explanation/load-test.md)
- [Monitoring architecture](explanation/monitoring-architecture-prometheus-grafana.md)

## History

- [Origins: 42 Berlin ft_transcendence](42-origins.md): the original defense write-up, kept as it was

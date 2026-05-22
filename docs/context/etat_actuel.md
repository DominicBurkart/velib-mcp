# État Actuel du Projet Velib MCP

Dernière mise à jour : 2026-04-23

## Statut global

Toutes les phases planifiées (0 à 4) sont terminées. Le serveur MCP est
fonctionnel, déployé via GitHub Actions vers Scaleway, et dispose d'une
couverture de tests unitaires et d'intégration automatisée.

| Phase | Description | Statut |
|-------|-------------|--------|
| 0 | Configuration projet, CI/CD, documentation | Terminée |
| 1 | Analyse des deux jeux de données Velib | Terminée |
| 2A | Fondation serveur et environnement | Terminée |
| 2B | Types MCP et protocole JSON-RPC 2.0 | Terminée |
| 3A | Client données et cache TTL | Terminée |
| 3B | Handlers MCP connectés aux données live | Terminée |
| 4 | Nettoyage repo (worktrees committés retirés) | Terminée |

## Interfaces livrées

- Tools MCP (implémentés dans `src/mcp/handlers.rs`) :
  `find_nearby_stations`, `get_station_by_code`,
  `search_stations_by_name`, `get_area_statistics`, `plan_bike_journey`.
- Resources MCP : `velib://stations/reference`,
  `velib://stations/realtime`, `velib://stations/complete`,
  `velib://health`.
- Transports (voir `src/mcp/server.rs`) : HTTP POST `/mcp` et WebSocket
  `/mcp/ws`.

## Évolutions en cours

Les améliorations itératives (qualité de code, couverture, clarté) sont
pilotées par PRs individuelles. Voir les issues ouvertes sur GitHub pour
la priorité actuelle.

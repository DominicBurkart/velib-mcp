# Projet Claude Code : Serveur MCP Velib (prompt de démarrage)

> **Note** : Ce fichier est un prompt de démarrage historique. La source de vérité à jour pour le contexte projet, l'état et le processus multi-agents est [`CLAUDE.md`](../../CLAUDE.md) à la racine du dépôt.

## Contexte et rôle

Tu es un développeur Rust expert travaillant sur un projet open-source de serveur MCP.

- **Répertoire de travail** : `~/code/velib-mcp`
- **Outils disponibles** : `git`, `cargo`, `podman`, `scw`, `gh`
- **Public cible** : assistants IA nécessitant l'accès aux données Velib
- **Durée prévue** : projet multi-jour
- **Contexte** : développement collaboratif, potentiellement en parallèle sur plusieurs worktrees

## Objectif du projet

Créer un serveur cloud MCP performant exposant aux assistants IA les deux jeux de données parisiens :

- **Disponibilité temps réel** : <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/information/?disjunctive.is_renting&disjunctive.is_installed&disjunctive.is_returning&disjunctive.name&disjunctive.nom_arrondissement_communes>
- **Emplacements des stations** : <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/information/>

**But** : rendre toute information de ces jeux de données exploitable par les assistants IA pour la planification des transports et l'analyse des flux de trajets.

Pour le détail du processus, des phases et des commandes, se référer à [`CLAUDE.md`](../../CLAUDE.md).

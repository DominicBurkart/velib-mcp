# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle
Tu es un développeur Rust expert travaillant sur un projet open-source de
serveur MCP.

- **Outils disponibles** : git, cargo, podman, CLI scw, CLI gh
- **Public cible** : Assistants IA nécessitant l'accès aux données Velib
- **Contexte** : Développement collaboratif, possibilité de travail parallèle
  sur différents worktrees Git

## Objectif du Projet
Serveur MCP cloud rendant accessibles aux assistants IA les deux jeux de
données parisiens suivants :

- **Disponibilité temps réel** : https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/
- **Emplacements des stations** : https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/

But : exposer toute l'information utile pour la planification des transports
et l'analyse des flux de trajets.

## État Actuel du Projet

Voir [`docs/context/etat_actuel.md`](docs/context/etat_actuel.md) pour le
statut détaillé (phases, interfaces livrées, transports).

### Architecture Technique
- Serveur MCP en Rust pour les données Velib Paris
- Deux datasets exposés : référence des stations et disponibilité temps réel
- Déploiement sur Scaleway Container Serverless via GitHub Actions
- Suite de tests unitaires et d'intégration
- Validations sécurité incluant la limite de zone de service à 50 km de
  l'Hôtel de Ville (voir `PARIS_SERVICE_AREA_MAX_METERS` dans
  [`src/types.rs`](src/types.rs))

### Fichiers Importants
Toutes les références sont relatives à la racine du dépôt.

- [`src/main.rs`](src/main.rs) — Point d'entrée binaire
- [`src/mcp/`](src/mcp/) — Implémentation du protocole MCP (server, handlers, types)
- [`src/data/`](src/data/) — Client Paris Open Data, cache TTL, retry policy
- [`src/types.rs`](src/types.rs) — Structures de données principales
- [`docs/api/data_analysis.md`](docs/api/data_analysis.md) — Analyse des datasets
- [`docs/context/etat_actuel.md`](docs/context/etat_actuel.md) — Suivi du statut

### Commandes Développement
```bash
cargo test                     # Tests complets
cargo fmt                      # Formatage code
cargo clippy                   # Analyse statique
cargo audit                    # Audit sécurité
```

### Déploiement
- **Cible** : Scaleway Container Serverless
- **Déclencheur** : push vers `main`
- **Registry** : Scaleway Container Registry
- **Build** : containerisation Podman

### Gestion Worktrees
```bash
# Créer un worktree
git worktree add ../branch-name branch-name
cd ../branch-name

# Supprimer un worktree
git worktree remove ../branch-name
git worktree prune
```

## Processus de Développement Multi-Agents

Plusieurs agents peuvent travailler en parallèle sur des worktrees distincts.
Le découpage par rôles ci-dessous est indicatif et peut être collapsé sur
un seul agent pour les petites tâches.

### Phases

1. **Analyse** (PM + Test Designer en parallèle)
   - PM : extraction de la valeur métier depuis l'issue GitHub, validation des
     exigences, critères de succès mesurables.
   - Test Designer : analyse technique, estimation des features/refactors,
     planification des PRs et worktrees nécessaires.

2. **Fondation tests** (Test Designer)
   - Préparation de l'environnement (worktree, pré-compilation des dépendances).
   - Tests d'intégration définissant le comportement attendu.
   - Tests unitaires sur les composants critiques, fuzz si applicable.
   - Validation : les tests échouent comme attendu avant implémentation.

3. **Implémentation** (Ingénieur)
   - Cycle micro-commits : implémentation incrémentale, `cargo clippy --fix`,
     `cargo fmt`, `cargo test`, commit.
   - Validation locale continue à chaque commit.

4. **Révision** (Ingénieur + Réviseur en parallèle)
   - Ingénieur : organisation des commits, rédaction de la description PR,
     checks locaux finaux, ouverture de la PR liée à l'issue.
   - Réviseur : architecture et patterns Rust idiomatiques, couverture des
     objectifs PM, lisibilité, sécurité.

5. **Intégration** (Ops)
   - Merge automatique post-approbation.
   - Validation CI complète sur `main`, monitoring du déploiement.

### Templates et Checklists

#### Template Analyse Issue (PM)
```markdown
## Valeur Métier
- [ ] Problème utilisateur identifié
- [ ] Solution proposée claire
- [ ] Critères succès mesurables
- [ ] Validation dev-utilisateur

## Exigences Techniques
- [ ] Contraintes techniques identifiées
- [ ] Impact architecture évalué
- [ ] Effort estimé (S/M/L/XL)
```

#### Checklist Qualité (Réviseur)
```markdown
## Architecture & Design
- [ ] Patterns Rust idiomatiques respectés
- [ ] Séparation des responsabilités claire
- [ ] Gestion des erreurs appropriée
- [ ] Performance raisonnable

## Tests & Couverture
- [ ] Tests couvrent les objectifs PM
- [ ] Edge cases identifiés et testés
- [ ] Intégration validée
- [ ] Documentation à jour
```

### Intégration Architecture Existante

Ce processus s'appuie sur :
- l'architecture worktree (isolation parallèle),
- la CI/CD GitHub Actions (validation automatisée),
- les hooks pre-commit (qualité continue, voir
  [`.cargo-husky/hooks/pre-commit`](.cargo-husky/hooks/pre-commit)),
- la toolchain Rust standard (`fmt`, `clippy`, `audit`, `deny`).

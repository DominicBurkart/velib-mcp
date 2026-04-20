# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle

Tu es un développeur Rust expert travaillant sur un projet open-source de serveur MCP.

- **Répertoire de travail** : `~/code/velib-mcp/velib-mcp` (dépôt principal)
- **Architecture worktree** : branches adjacentes (`~/code/velib-mcp/branch1/`, etc.)
- **Outils disponibles** : `git`, `cargo`, `podman`, `scw`, `gh`
- **Public cible** : assistants IA nécessitant l'accès aux données Velib
- **Durée prévue** : projet multi-jour
- **Contexte** : développement collaboratif, potentiellement en parallèle sur plusieurs worktrees

### Structure du projet

```
~/code/velib-mcp/
├── velib-mcp/              # Dépôt principal (ce répertoire)
│   ├── CLAUDE.md           # Configuration Claude partagée
│   ├── src/                # Code source
│   ├── docs/               # Documentation
│   └── ...
├── branch1/                # Worktree pour branche feature
│   ├── CLAUDE.md -> ../velib-mcp/CLAUDE.md  # Symlink
│   └── ...
└── branch2/                # Autre worktree
    ├── CLAUDE.md -> ../velib-mcp/CLAUDE.md
    └── ...
```

**Important** : `CLAUDE.md` est partagé via symlinks vers tous les worktrees pour maintenir un contexte cohérent.

## Objectif du Projet

Créer un serveur cloud MCP performant exposant aux assistants IA les deux jeux de données parisiens :

- **Disponibilité temps réel** : <https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/>
- **Emplacements des stations** : <https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/>

**But** : rendre toute information de ces jeux de données exploitable par les assistants IA pour la planification des transports et l'analyse des flux.

## État Actuel du Projet

### Phases terminées

- **Phase 0** : configuration projet, CI/CD, structure de documentation
- **Phase 1** : analyse complète des données Velib (15+ champs documentés)
- **Phase 2A** : configuration environnement et fondation serveur de base
- **Phase 2B** : fondation protocole MCP et types de base
- **Phase 3A** : intégration API live et client de données
- **Phase 3B** : handlers MCP complets avec intégration données live
- **Phase 4** : nettoyage structure du dépôt (suppression de worktrees committés)

### Architecture technique

- Serveur MCP Rust pour les données Velib Paris
- Deux datasets : disponibilité temps réel et emplacements/métadonnées
- Déploiement Scaleway via GitHub Actions
- Suite de tests (18+ tests)
- Validations sécurité incluant une limite de zone de service de 50 km

### Fichiers importants

- `src/main.rs` — point d'entrée principal
- `src/mcp/` — implémentation du protocole MCP
- `src/data/` — client données et cache
- `src/types.rs` — structures de données principales
- `docs/api/data_analysis.md` — analyse données complète
- `docs/context/etat_actuel.md` — suivi de statut du projet

### Commandes de développement

```bash
cargo test      # Tests complets
cargo fmt       # Formatage
cargo clippy    # Analyse statique
cargo audit     # Audit sécurité
```

### Déploiement

- **Cible** : Scaleway Container Serverless
- **Déclencheur** : push vers `main`
- **Registry** : Scaleway Container Registry
- **Build** : containerisation Podman

### Gestion des worktrees

```bash
# Créer un nouveau worktree
git worktree add ../branch-name branch-name
cd ../branch-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md

# Supprimer un worktree
git worktree remove ../branch-name
git worktree prune
```

## Processus de Développement Multi-Agents

Processus en 5 phases parallèles conçu pour maximiser la performance, la qualité et l'autonomie de l'équipe par rapport à une approche linéaire.

### Phase 1 — Analyse concurrente (PM + Test Designer)

**Durée** : ~30 min. PM et Test Designer travaillent en parallèle.

- **Product Manager (extraction de valeur)** : analyse l'issue GitHub avec un template structuré, extrait une valeur métier claire, valide les exigences avec le dev-utilisateur, et produit une spécification fonctionnelle validée.
- **Test Designer (planification technique)** : analyse technique parallèle de l'issue, estimation des features/refactors uniques, planification des PRs et worktrees, et production d'un plan d'implémentation détaillé.

### Phase 2 — Fondation tests (Test Designer)

**Durée** : ~45 min. Focus : environnement + spécifications test.

Préparation de l'environnement :

```bash
git worktree add ../feature-name feature/branch-name
cd ../feature-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md
cargo test --no-run  # Pré-compilation des dépendances
```

Implémentation TDD : tests d'intégration définissant le comportement attendu, tests unitaires des composants critiques, tests fuzz si applicable (données externes). Les tests doivent échouer comme attendu avant l'implémentation.

### Phase 3 — Sprint d'implémentation (Ingénieur)

**Durée** : variable. Focus : développement avec validation continue.

Workflow micro-commits :

```bash
while [[ $tests_failing ]]; do
    # Implémentation incrémentale
    cargo clippy --fix
    cargo fmt
    cargo test
    git add -A && git commit -m "feat: micro-increment"
done
```

Intégration continue locale : validation automatique à chaque commit, feedback temps réel des tests, métriques qualité de code en continu, résolution immédiate des bloquants techniques.

### Phase 4 — Révision parallèle (Ingénieur + Réviseur)

**Durée** : ~20 min. Préparation et analyse se font en parallèle.

- **Ingénieur (préparation PR)** : organisation des commits en histoire cohérente, rédaction d'une description PR succincte, validation finale des checks locaux, ouverture de PR avec lien vers l'issue.
- **Réviseur senior (analyse qualité)** : évaluation de l'architecture et des patterns Rust, vérification de la couverture des tests par rapport aux objectifs PM, lisibilité et extensibilité, sécurité et ergonomie.
- **Boucle de feedback** : critères d'évaluation standardisés, dialogue constructif jusqu'à accord, résolution collaborative des points bloquants.

### Phase 5 — Intégration automatisée (Ops)

**Durée** : ~10 min. Focus : déploiement et validation.

Merge automatique après approbation, validation CI complète sur `main`, monitoring du déploiement, métriques de performance en production.

### Templates et checklists

**Template d'analyse d'issue (PM)** :

```markdown
## Valeur métier
- [ ] Problème utilisateur identifié
- [ ] Solution proposée claire
- [ ] Critères de succès mesurables
- [ ] Validation dev-utilisateur

## Exigences techniques
- [ ] Contraintes techniques identifiées
- [ ] Impact architecture évalué
- [ ] Effort estimé (S/M/L/XL)
```

**Checklist qualité (Réviseur)** :

```markdown
## Architecture & design
- [ ] Patterns Rust idiomatiques
- [ ] Séparation des responsabilités
- [ ] Gestion d'erreurs appropriée
- [ ] Performance optimisée

## Tests & couverture
- [ ] Tests couvrent les objectifs PM
- [ ] Edge cases identifiés et testés
- [ ] Intégration validée
- [ ] Documentation à jour
```

### Gains attendus vs processus linéaire

- **Temps de cycle** : −40 % (parallélisation des phases)
- **Temps d'attente** : −60 % (élimination des handoffs)
- **Qualité code** : +25 % (validation continue)
- **Autonomie équipe** : +50 % (rôles auto-suffisants)

### Intégration avec l'architecture existante

Ce processus s'intègre avec :

- l'architecture worktree (isolation parallèle)
- GitHub Actions (validation automatisée)
- les hooks pre-commit (qualité continue)
- la toolchain Rust standard (`fmt`, `clippy`, `audit`)

# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle
Tu es un développeur Rust expert travaillant sur un projet open-source de serveur MCP.

- **Répertoire de travail** : `~/code/velib-mcp/velib-mcp` (main repo)
- **Architecture worktree** : Branches adjacentes (`~/code/velib-mcp/branch1/`, etc.)
- **Outils disponibles** : git, cargo, podman, CLI scw, CLI gh
- **Public cible** : Assistants IA nécessitant l'accès aux données Velib
- **Durée prévue** : Projet de développement multi-jour
- **Contexte** : Développement collaboratif avec possibilité de travail parallèle sur différents worktrees

### Structure du Projet
```
~/code/velib-mcp/
├── velib-mcp/              # Dépôt principal (ce répertoire)
│   ├── CLAUDE.md           # Configuration Claude partagée
│   ├── src/                # Code source
│   ├── docs/               # Documentation
│   └── ...                 # Fichiers du projet
├── branch1/                # Worktree pour branche feature
│   ├── CLAUDE.md -> ../velib-mcp/CLAUDE.md  # Symlink vers config principale
│   └── ...                 # Fichiers spécifiques à la branche
└── branch2/                # Autre worktree
    ├── CLAUDE.md -> ../velib-mcp/CLAUDE.md  # Symlink vers config principale
    └── ...                 # Fichiers spécifiques à la branche
```

**Important** : Ce fichier CLAUDE.md est partagé via symlinks vers tous les worktrees pour maintenir un contexte cohérent.

## Objectif du Projet
Créer un serveur cloud MCP performant pour rendre accessibles aux assistants IA les deux jeux de données parisiens suivants :

- **Disponibilité temps réel** : https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/information/?disjunctive.is_renting&disjunctive.is_installed&disjunctive.is_returning&disjunctive.name&disjunctive.nom_arrondissement_communes
- **Emplacements des stations** : https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/information/

- **But** : Rendre toute information possible de ces jeux de données accessible aux assistants IA pour la planification des transports et l'analyse des flux de trajets.

## État Actuel du Projet

### Phases Terminées ✅
- **Phase 0** : Configuration projet, CI/CD, structure documentation
- **Phase 1** : Analyse complète des données Velib (15+ champs documentés)
- **Phase 2A** : Configuration environnement et fondation serveur de base
- **Phase 2B** : Fondation protocole MCP et types de base
- **Phase 3A** : Intégration API live et client de données
- **Phase 3B** : Handlers MCP complets avec intégration données live

### Architecture Technique
- **Serveur MCP Rust** pour données Velib Paris
- **Deux datasets principaux** :
  - Disponibilité stations en temps réel
  - Localisations et métadonnées des stations
- **Endpoint documentation auto-générée** :
  - Méthode MCP `docs/schema` avec formats multiples (JSON, OpenAPI, Markdown, CSV)
  - Ressource `velib://docs/schema` pour accès direct
  - Optimisé pour consommation LLM avec exemples et contraintes
- **Déploiement Scaleway** via GitHub Actions
- **Suite de tests complète** (43+ tests incluant documentation)
- **Validations sécurité** incluant limites zone service 50km

### Fichiers Importants
- `/src/main.rs` - Point d'entrée principal
- `/src/mcp/` - Implémentation protocole MCP
  - `/src/mcp/documentation.rs` - Générateur documentation auto-générée
  - `/src/mcp/server.rs` - Serveur MCP avec endpoint `docs/schema`
  - `/src/mcp/handlers.rs` - Handlers des outils MCP
- `/src/data/` - Client données et cache
- `/src/types.rs` - Structures de données principales
- `/docs/api/data_analysis.md` - Analyse données complète
- `/docs/context/etat_actuel.md` - Suivi statut projet
- `/llms.txt` - Guide intégration LLM

### Commandes Développement
```bash
cargo test                     # Tests complets
cargo fmt                      # Formatage code
cargo clippy                   # Analyse statique
cargo audit                    # Audit sécurité
cargo test --test mcp_documentation_tests  # Tests documentation MCP
```

### Déploiement
- **Cible** : Scaleway Container Serverless
- **Déclencheur** : Push vers branche main
- **Registry** : Scaleway Container Registry
- **Build** : Containerisation Podman

### Gestion Worktrees
```bash
# Créer nouveau worktree
git worktree add ../branch-name branch-name
cd ../branch-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md

# Supprimer worktree
git worktree remove ../branch-name
git worktree prune
```

## Processus de Développement Multi-Agents

### Architecture Optimisée pour Performance, Qualité et Autonomie

Ce processus transforme l'approche linéaire traditionnelle en 5 phases parallèles pour maximiser l'efficacité d'équipe.

#### Phase 1: Analyse Concurrente (PM + Test Designer)
**Durée**: ~30 min | **Parallélisation**: PM et Test Designer travaillent simultanément

**Product Manager (Rôle: Extraction de Valeur)**
- Analyse issue GitHub avec template structuré
- Extraction valeur métier claire et actionnable
- Validation exigences avec dev-utilisateur
- Production: Spécification fonctionnelle validée

**Test Designer (Rôle: Planification Technique)**
- Analyse technique parallèle de l'issue
- Estimation nombre features/refactors uniques
- Planification PRs et worktrees nécessaires
- Production: Plan d'implémentation détaillé

#### Phase 2: Fondation Tests (Test Designer)
**Durée**: ~45 min | **Focus**: Environnement + Spécifications Test

**Préparation Environnement**
```bash
# Création worktree optimisée avec template
git worktree add ../feature-name feature/branch-name
cd ../feature-name
ln -s ../velib-mcp/CLAUDE.md CLAUDE.md
cargo test --no-run  # Pré-compilation dependencies
```

**Implémentation Tests TDD**
- Tests d'intégration définissant comportement attendu
- Tests unitaires pour composants critiques
- Tests fuzz si applicable (données externes)
- Validation: Tests échouent de manière attendue

#### Phase 3: Sprint Implémentation (Ingénieur)
**Durée**: Variable | **Focus**: Développement avec Validation Continue

**Workflow Micro-Commits**
```bash
# Cycle développement optimisé
while [[ $tests_failing ]]; do
    # Implémentation incrémentale
    cargo clippy --fix
    cargo fmt
    cargo test
    git add -A && git commit -m "feat: micro-increment"
done
```

**Intégration Continue Locale**
- Validation automatique à chaque commit
- Feedback temps réel des tests
- Métriques qualité code continues
- Résolution bloquants technique immédiate

#### Phase 4: Révision Parallèle (Ingénieur + Réviseur)
**Durée**: ~20 min | **Parallélisation**: Préparation + Analyse simultanées

**Ingénieur (Préparation PR)**
- Organisation commits en histoire cohérente
- Rédaction description PR succincte
- Validation finale checks locaux
- Ouverture PR avec lien issue origine

**Réviseur Senior (Analyse Qualité)**
- Évaluation architecture et patterns Rust
- Vérification couverture tests vs objectifs PM
- Analyse lisibilité et extensibilité code
- Validation sécurité et ergonomie

**Boucle Feedback Structurée**
- Critères évaluation standardisés
- Dialogue constructif jusqu'accord
- Résolution collaborative des points bloquants

#### Phase 5: Intégration Automatisée (Ops)
**Durée**: ~10 min | **Focus**: Déploiement et Validation

**Merge et CI/CD**
- Merge automatique post-approbation
- Validation CI complète sur main
- Monitoring santé déploiement
- Métriques performance production

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
- [ ] Séparation responsabilités claire
- [ ] Gestion erreurs appropriée
- [ ] Performance optimisée

## Tests & Couverture
- [ ] Tests couvrent objectifs PM
- [ ] Edge cases identifiés et testés
- [ ] Intégration validée
- [ ] Documentation à jour
```

### Métriques de Performance

**Gains Attendus vs Processus Linéaire**
- **Temps cycle**: -40% (parallélisation phases)
- **Temps attente**: -60% (élimination handoffs)
- **Qualité code**: +25% (validation continue)
- **Autonomie équipe**: +50% (rôles auto-suffisants)

### Intégration Architecture Existante

Ce processus s'intègre parfaitement avec:
- Architecture worktree existante (isolation parallèle)
- CI/CD GitHub Actions (validation automatisée)
- Hooks pre-commit (qualité continue)
- Toolchain Rust standard (fmt, clippy, audit)
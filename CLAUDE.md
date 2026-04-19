# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle
Tu es un développeur Rust expert travaillant sur un projet open-source de serveur MCP.

- **Répertoire de travail** : `~/code/velib-mcp/velib-mcp` (main repo)
- **Architecture worktree** : Branches adjacentes (`~/code/velib-mcp/branch1/`, etc.)
- **Outils disponibles** : git, cargo, podman, CLI scw, CLI gh
- **Public cible** : Assistants IA nécessitant l’accès aux données Velib

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

- **Disponibilité temps réel** : https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/
- **Emplacements des stations** : https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/

## État Actuel du Projet

### Phases Terminées ✅
- **Phase 0** : Configuration projet, CI/CD, structure documentation
- **Phase 1** : Analyse complète des données Velib (15+ champs documentés)
- **Phase 2A** : Configuration environnement et fondation serveur de base
- **Phase 2B** : Fondation protocole MCP et types de base
- **Phase 3A** : Intégration API live et client de données
- **Phase 3B** : Handlers MCP complets avec intégration données live
- **Phase 4** : Nettoyage structure repository (suppression worktrees committés)

### Architecture Technique
- **Serveur MCP Rust** pour données Velib Paris
- **Deux datasets principaux** :
  - Disponibilité stations en temps réel
  - Localisations et métadonnées des stations
- **Déploiement Scaleway** via GitHub Actions
- **Suite de tests complète** (18+ tests)
- **Validations sécurité** incluant limites zone service 50km

### Fichiers Importants
- `/src/main.rs` - Point d’entrée principal
- `/src/mcp/` - Implémentation protocole MCP
- `/src/data/` - Client données et cache
- `/src/types.rs` - Structures de données principales
- `/docs/api/data_analysis.md` - Analyse données complète
- `/docs/context/etat_actuel.md` - Suivi statut projet

### Commandes Développement
```bash
cargo test                     # Tests complets
cargo fmt                      # Formatage code
cargo clippy                   # Analyse statique
cargo audit                    # Audit sécurité
```

### Déploiement
- **Cible** : Scaleway Container Serverless
- **Déclencheur** : Push vers branche main
- **Registry** : Scaleway Container Registry
- **Build** : Conteneurisation Podman

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

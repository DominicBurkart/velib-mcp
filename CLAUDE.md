# Projet CLAUDE : Serveur MCP Velib

## Contexte et Rôle
Développeur Rust expert sur un projet open-source de serveur MCP donnant
accès aux données Velib Paris pour assistants IA.

- **Outils** : git, cargo, podman, CLI `scw`, CLI `gh`
- **Public cible** : Assistants IA consommant des données Velib
- **Worktrees** : support des branches adjacentes via `git worktree`. Si un
  worktree est créé, le `CLAUDE.md` racine est partagé via symlink pour
  préserver le contexte.

## Objectif du Projet
Serveur cloud MCP performant exposant les deux jeux de données Velib
publiés par la Ville de Paris :

- [Disponibilité temps réel](https://opendata.paris.fr/explore/dataset/velib-disponibilite-en-temps-reel/)
- [Emplacements des stations](https://opendata.paris.fr/explore/dataset/velib-emplacement-des-stations/)

Le but est de rendre l'intégralité de ces jeux exploitable par des
assistants IA pour la planification de trajets et l'analyse des flux.

## État du Projet

Les phases planifiées (0 à 4) sont terminées. Pour le détail des
livrables, des interfaces MCP exposées et des évolutions en cours, voir
[`docs/context/etat_actuel.md`](docs/context/etat_actuel.md).

### Repères dans le code
- `src/main.rs` — point d'entrée
- `src/mcp/` — implémentation du protocole MCP
- `src/data/` — client API et cache TTL
- `src/types.rs` — structures de données partagées
- `docs/api/data_analysis.md` — analyse complète des datasets
- `docs/api/mcp_interface_spec.md` — spécification des tools et resources

## Commandes Développement

```bash
cargo test                                            # tests complets
cargo fmt --all                                       # formatage
cargo clippy --all-targets --all-features -- -D warnings
cargo audit                                           # audit sécurité
cargo deny check licenses bans sources                # conformité licences
```

Le hook `pre-commit` installé par `cargo-husky` reproduit `clippy --fix`,
`fmt`, `cargo sort` et `cargo deny`. Un échec non nul du hook avorte le
commit.

## Déploiement
- **Cible** : Scaleway Container Serverless
- **Déclencheur** : push sur `main`
- **Registry** : Scaleway Container Registry
- **Build** : containerisation Podman (image distroless Debian)

## Gestion des Worktrees

```bash
# Créer un worktree pour une branche feature
git worktree add ../branch-name feature/branch-name
ln -s ../velib-mcp/CLAUDE.md ../branch-name/CLAUDE.md

# Supprimer un worktree
git worktree remove ../branch-name
git worktree prune
```

## Processus de Développement

Le flux typique pour une issue est :

1. **Analyse** — extraire la valeur métier, identifier les contraintes
   techniques et estimer l'effort (S/M/L/XL).
2. **Tests d'abord** — écrire les tests d'intégration et unitaires
   décrivant le comportement attendu ; vérifier qu'ils échouent.
3. **Implémentation incrémentale** — micro-commits validés localement par
   `cargo clippy`, `cargo fmt`, `cargo test`.
4. **Revue** — ouvrir une PR liée à l'issue, vérifier patterns Rust
   idiomatiques, couverture, lisibilité, sécurité.
5. **Merge et CI** — après approbation, le merge sur `main` déclenche le
   pipeline GitHub Actions et le déploiement Scaleway.

### Checklist Qualité (Revue)
- Patterns Rust idiomatiques et séparation des responsabilités
- Gestion d'erreur explicite (pas de `unwrap` en chemin chaud)
- Couverture des cas limites
- Documentation mise à jour si l'API publique change
- Validations sécurité (ex. zone de service 50 km) préservées

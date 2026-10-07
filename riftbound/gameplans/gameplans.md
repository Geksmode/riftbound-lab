# Gameplans Akali (Gorica) et LeBlanc (Deathknell) : synthèse (2026-10-03)

Sources : guide Metafy de Gorica (résumé, texte non reproduit), deck tech de Gorica (Turn 'em Sideways), parties
commentées BMU et RQ Singapour pour Akali ; 50k Comics (Houston), DZiden, RQ Singapour, guides écrits pour LeBlanc.
Détail et horodatages : [notes_akali_videos.md](notes_akali_videos.md), [notes_leblanc_videos.md](notes_leblanc_videos.md).
Traduction en code : `engine/plans.py`.

## Akali Heron (Gorica)
**Identité.** Deck contrôle lent orienté valeur. Les points du début ne comptent pas, il faut ne pas perdre.
Moteur : Stellacorn Herder (pioche en bougeant) + légende non empowered (Retreat le ramène) + Astral Heron.

- **Mulligan** : renvoyer 2 cartes sauf main avec Stellacorn ET Heron ; ne jamais renvoyer d'unité.
- **Battlefield** : Void Gate à l'aveugle ; Sigil of the Storm gardé pour les parties où LeBlanc est sur Windswept.
- **Légende** : ne pas l'empower ; elle sert à rappeler Herder (pioche) ou Deadly Weapon.
- **Heron** : sur un battlefield seulement s'il y a une carte cachée à côté pour la déclencher, sinon à la base.
- **Ne jamais engager toutes ses unités** : garder un défenseur à la base.

### Contre LeBlanc (mise à jour Metafy, Gorica le donne 45/55)
- Jouer **agressif en milieu de partie** au lieu d'attendre le moteur.
- Mulligan pour Kai'Sa, Darius, Blitzcrank, Shuriken Flip, Falling Star et l'interaction.
- **Empower Deadly Weapon** pour tuer les unités que LeBlanc laisse sur les battlefields.
- **Tuer Karthus à vue.** Se méfier de Vi en embuscade.
- Après la partie 1 : Akali, Silent rentre.

## LeBlanc Deathknell
**Deux versions** (riftbound.gg) : la version chinoise, lente, avec plus de removal et Dusk Rose Lab ; la version
occidentale, **tempo autour de Baited Hook**, qui fait boule de neige en milieu de partie. La liste IQ #5 jouée
dans les simulations est la version Hook, « très proactive ».

**Identité commune.** Échanger les petites unités tôt (chaque mort pioche ou rampe). Le Deceiver ne produit rien
sans conquête ou tenue : la version Hook doit occuper les battlefields et faire marquer sa première vraie unité.

- **Battlefield** : Windswept Hillock en commençant (le Reflet gank l'autre battlefield au tour 2), Star Spring sinon.
- **Mulligan** : un 2-drop en commençant ; rendre Dragon, le 2e Karthus, souvent Hidden Blade.
- **Tour 1** : toujours un corps (Scout, Dignitary, Fragmented). **Tour 2** : conquérir et cloner.
- **Karthus** reste à la base, après les corps Deathknell. Glasc est la cible de copie favorite.
- **Légende** : défausser Sacrifice, Dignitary, Hook en trop, Dragon ; jamais Sentry, Karthus, Glasc, Rex.
  Ne pas cloner si cela force à jeter une carte clé.
- **Baited Hook en échelle** : Glasc → Rex/Dragon → Herald/Watcher. Sacrifice en réponse à un retrait.

### Pour Akali, contrer LeBlanc (RiftStorm, Metafy de Gorica)
- Refuser la conquête ou la tenue ; tuer Karthus à vue.
- Viser l'unité que copie le Reflet, pas le Reflet (il meurt seul au début du tour).
- Ne pas attaquer Fragmented pour rien : chaque mort lui fait piocher.

## Résultats (moteur fidèle, G2 contre LeBlanc IQ #5, 400 parties par configuration)
| Akali \ LeBlanc | sans plan | Deathknell | Hook tempo | Hook tempo, toujours Windswept |
|---|---|---|---|---|
| sans plan | 48,8 % | 45,2 % | 43,2 % / 43,8 %* | |
| plan moteur (Gorica) | | 57,8 % | 51,5 % / **54,2 %*** | 53,2 %* |
| plan agressif (Metafy vs LeBlanc) | 57,8 % | 56,0 % | 47,2 % / 50,7 %* | 45,2 %* |

\* graines neuves 80000+, les autres 70000+. ± 2,5 points par case ; comparaisons appariées (mêmes graines) :
- Hook tempo est le meilleur plan LeBlanc (−6 ± 3 pour Akali par rapport à Deathknell).
- Plan moteur : +8 à +10 points par rapport à Akali sans plan (écart apparié, même sens sur deux blocs).
- **Niveaux absolus : ± 5 points, pas ± 2,5.** Les blocs de 400 parties varient plus que le hasard binomial
  (fil « Agent manager », session 2 : plan moteur contre Hook tempo Windswept = 53,3 / 40,0 / 47,3 % sur trois
  blocs, moyenne 46,8 % sur 1 200 parties, cause inconnue). Seuls les écarts appariés dans un même bloc sont fiables.
- Plan agressif : jamais meilleur ; nettement moins bon si LeBlanc présente toujours Windswept (−8,0 ± 3,3).
- Windswept systématique ne coûte rien au plan moteur (−1 ± 2), car il répond avec Sigil of the Storm.
- Le réel reste plus bas (~30-35 % selon riftdecks, 45/55 selon Gorica) : la recherche courte joue LeBlanc moins bien qu'un humain.

## Tempo et ressources (guides, 3 oct.)
Sources : riftbound.gg « The in-depth guide to scoring » (texte lu) ; riftbound.zone et learnriftbound.gg
« Tempo vs Value », Ultimate Guard « three concepts » (résumés de recherche, sites bloqués ici). Vidéos YouTube non
regardées : elles demandent ton PC.

- **Tempo** = contrôler le plateau et les battlefields maintenant. **Valeur** = cartes et unités pour plus tard.
  Une carte qui ne change pas le prochain score ou la prochaine tenue est de la valeur, pas du tempo.
- **Le dernier point** ne se prend en conquérant que si l'on a déjà marqué tous les autres battlefields ce tour-là
  (sinon on pioche). On gagne donc à 7 par une **tenue**, ou à 6 en conquérant les deux battlefields.
- **Zone létale : 6 points.** À partir de là il n'y a plus de long terme pour l'adversaire : chaque tour où il ne
  conteste pas peut être le dernier.
- **Conquérant ou teneur.** Le deck tempo vise 2-4-6 puis une double conquête ; le deck contrôle retarde
  l'adversaire d'un tour (le laisser sur un score impair) puis gagne en tenant à 7. Akali Heron est un teneur.
- **Refuser la tenue gagnante** passe avant toute conquête ailleurs : un battlefield tenu par un adversaire à 7
  doit être attaqué, même à perte, même avec Karthus. C'est exactement l'erreur de LeBlanc au tour 13 de la partie
  « Contre Windswept, LeBlanc commence » : elle a pris Sigil au lieu d'attaquer le Mech de Windswept.
- **L'erreur la plus courante est d'attendre** : garder des unités « pour plus tard » quand l'adversaire est à 6.

### Dans le moteur (ai.py, `TEMPO`, désactivable avec RB_TEMPO=0)
- Une victoire vaut plus si elle arrive tôt, une défaite coûte moins si elle arrive tard.
- Contrôler un battlefield à 7 points vaut +40 (tenue gagnante imminente), pour soi comme pour l'adversaire.
- En zone létale adverse (6+), ou si un seul tirage dit que tout perd, chaque option est rejouée 6 fois :
  l'IA choisit la ligne qui survit le plus souvent au lieu d'un choix au hasard entre défaites.
- En zone létale adverse, les malus du plan (« Karthus reste à la base »…) sont levés.
- La politique rapide des simulations attaque d'abord le battlefield de la tenue gagnante adverse.

Effet mesuré (graines 90000+, 400 parties, ancienne et nouvelle IA sur les mêmes donnes, les deux camps changent) :
| Akali \ LeBlanc | ancienne IA | nouvelle IA | écart apparié |
|---|---|---|---|
| sans plan vs Hook tempo | 42,0 % | 47,0 % | +5,0 ± 2,4 |
| plan moteur vs Hook tempo | 51,8 % | 52,2 % | +0,5 ± 2,3 (on ne sait pas) |
| plan moteur vs Hook tempo Windswept | 51,5 % | 48,5 % | −3,0 ± 2,3 (on ne sait pas) |
Les parties raccourcissent (14,2 → 13,4 tours). Défaut restant vu au replay (graine 90003) : Akali à 7 finit
ses tours sans contester quand LeBlanc reprend les battlefields.

### LeBlanc comme les vrais joueurs (remarques de l'utilisateur et deux vidéos, 3 oct.)
- Le Reflet et l'unité copiée restent sur le battlefield (rappel volontaire pénalisé) ; LeBlanc cache une carte là
  où il tient, surtout avec un Reflet seul (RB_REFL=0 pour l'ancien comportement). Rappels : 103 → 61 sur 80 parties ;
  cartes cachées : 14 % → 38 % des occasions.
- Vidéos (notes_leblanc_videos.md, RB_VIDEO=0 pour couper) : copier Ruined Rex / Watcher en priorité, défausser un
  Karthus en double si Glasc peut le ramener, Watcher closer, le Reflet prêt va prendre l'autre battlefield.
- Effet pour Akali (plan moteur, 400 parties appariées) : Reflet +1,8 ± 2,1 (Hook tempo) et +0,2 ± 1,9 (Windswept) ;
  vidéos −3,0 ± 1,5 (graines 91000+) mais +0,2 ± 1,3 sur graines neuves 92000+. **On ne sait pas** : aucun effet
  mesurable sur le taux de victoire, le jeu de LeBlanc est seulement plus fidèle.

## Ce que le moteur ne sait pas reproduire
Les plans sont des bonus/malus ajoutés à une recherche courte (2 tours), pas des lignes jouées à la main.
Le bluff, la lecture de la main adverse et l'ordre fin des Hooks restent hors de portée.

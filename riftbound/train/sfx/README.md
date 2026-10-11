# Sons de la table

Ambiance par défaut `cartes` : vrais bruits de cartes du paquet Kenney « Casino Audio » (CC0, `cartes/LICENSE-KENNEY.txt`),
importés par `bash importer_kenney.sh <zip>` (MP3 mono, variantes numérotées, `cartes/liste.json`). Correspondance
« événement → son » : table `CARTES` de `SFX` dans `src.html`.

Fichiers MP3 tirés du paquet npm `uisfx` 0.4.0 (audio CC0 1.0, voir `LICENSE-AUDIO` ; code MIT non repris) : 6 autres ambiances
(`studio`, `cinematic`, `zen`, `soft`, `organic`, `glass`) et 21 sons chacune. La correspondance
« événement de la table → fichier » est dans `src.html` (`SFX`, table `MAP`). `build.sh` copie ce dossier dans `build/sfx/`.
Pour remplacer un son : garder le même nom de fichier dans chaque ambiance.

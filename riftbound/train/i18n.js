// Version anglaise de la table (Riftbound Observatory). src.html reste écrit en français ; en anglais, ce fichier traduit
// le texte au moment où il entre dans la page : nœuds texte et attributs (title, aria-label, placeholder, label),
// dès qu'un innerHTML / textContent / insertAdjacentHTML est posé (avant toute mesure de mise en page), et un
// MutationObserver rattrape le reste. Le moteur parle anglais de lui-même (train.set_lang, engine/lang.py).
//
// Langue : ?lang=en ou ?lang=fr dans l'adresse, sinon le choix gardé (clé rbt-lang), sinon le français.
// I18N.set("en") garde le choix et recharge la page (le moteur repart dans la bonne langue, la partie en cours est
// gardée dans le navigateur comme d'habitude).
//
// Dictionnaire : D = [[français, anglais], ...]. Le texte est comparé sans ses espaces de bord (gardés tels quels) et avec
// les espaces internes réduits à un seul. ${} marque une partie variable : elle est elle-même traduite si elle est dans
// le dictionnaire, et reprise dans l'ordre (${2} pour changer l'ordre). P = motifs écrits à la main (regex → texte ou
// fonction). Une chaîne française absente laisse le texte tel quel : verif_anglais.mjs les cherche dans toute la table.
(function () {
  "use strict";
  let stored = null;
  try { stored = localStorage.getItem("rbt-lang"); } catch (e) {}
  const q = new URLSearchParams(location.search).get("lang");
  const lang = q === "en" || q === "fr" ? q : stored === "en" ? "en" : "fr";
  if (q === "en" || q === "fr") { try { localStorage.setItem("rbt-lang", q); } catch (e) {} }

  const D = [
    // ---------------------------------------------------------------- barre du haut, plateau, boutons
    ["Choisis ton deck et celui de l'IA, puis joue.", "Choose your deck and the AI's, then play."],
    ["☰ Menu", "☰ Menu"], ["Nouvelle partie", "New game"], ["Journal", "Log"], ["Sons de la table", "Table sounds"],
    ["Points", "Points"], ["Chargement du moteur de jeu…", "Loading the game engine…"],
    ["Conseil", "Hint"], ["Ce que jouerait l'IA à ta place", "What the AI would play in your place"],
    ["↶ Reprendre", "↶ Undo"], ["Annule ton dernier coup", "Takes back your last move"],
    ["Chaîne en cours", "Current chain"], ["Journal de la partie", "Game log"], ["Fermer le journal", "Close the log"],
    ["Menu principal", "Main menu"], ["Éditeur de deck", "Deck editor"], ["Afficher le défi du tutoriel", "Show the tutorial challenge"],
    ["Touche une carte à cadre vert pour voir ce qu'elle peut faire, ou glisse-la vers sa zone ou sa cible. Les choix (cibles, effets optionnels) se font sur la table, la question s'affiche au milieu du plateau. Touche une défausse pour voir son contenu. Flèches corail : coups de l'IA.",
     "Tap a card with a green frame to see what it can do, or drag it to its zone or target. Choices (targets, optional effects) are made on the table; the question shows in the middle of the board. Tap a trash pile to see its contents. Coral arrows: the AI's moves."],
    ["Fermer", "Close"], ["le moteur s'est arrêté", "the engine stopped"],
    ["Terminer le tour", "End turn"], ["Passer", "Pass"], ["Attends ton tour", "Wait for your turn"],
    ["Réponds au choix", "Answer the choice"], ["À toi", "Your move"], ["choix en cours", "choice pending"],
    ["${} joue…", "${} is playing…"], ["En attente de ${}…", "Waiting for ${}…"], ["tour ${}", "turn ${}"],
    ["tour ${} · ${}", "turn ${} · ${}"], ["Tour de ${}", "${}'s turn"],
    ["réaction", "reaction"], ["Déclenchement", "Trigger"], ["Sur la chaîne", "On the chain"], ["Chaîne vide", "Empty chain"],
    ["Partie gardée dans ce navigateur", "Game kept in this browser"], ["Partie non enregistrée", "Game not saved"],
    ["Partie enregistrée ✓", "Game saved ✓"],
    ["énergie flottante", "floating energy"], ["puissance flottante ${}", "floating power ${}"],
    ["Rune Pool : ressources gagnées et pas encore dépensées. L'énergie d'une rune épuisée avant d'être recyclée reste utilisable jusqu'à la fin du tour.",
     "Rune Pool: resources gained and not spent yet. The energy of a rune exhausted before being recycled stays usable until the end of the turn."],
    ["Carte cachée", "Hidden card"], ["révélée", "revealed"], ["révélée à l'adversaire", "revealed to the opponent"],
    ["Ta main", "Your hand"], ["(épuisée)", "(exhausted)"], ["Ta base", "Your base"], ["Base de ${}", "${}'s base"],
    ["Deck : ${} cartes. Dessus connu (Predict, Vision) : ${}. Touche pour voir.", "Deck: ${} cards. Known top (Predict, Vision): ${}. Tap to see."],
    ["Deck : ${} cartes", "Deck: ${} cards"], ["Défausse : touche pour voir les ${} cartes", "Trash: tap to see the ${} cards"],
    ["Bannies : touche pour voir les ${} cartes", "Banished: tap to see the ${} cards"], ["Bann.", "Ban."],
    ["JOUABLE", "PLAYABLE"], ["BANNIE", "BANISHED"],
    ["La plus récente en premier. Zone publique (règle 108.6.e). Une carte « JOUABLE » se joue depuis le bout de ta main, en payant son coût.",
     "Most recent first. Public zone (rule 108.6.e). A \u201cPLAYABLE\u201d card is played from the end of your hand, paying its cost."],
    ["Prêtes", "Ready"], ["Réserve ${}", "Reserve ${}"], ["Runes", "Runes"], ["Base", "Base"], ["Légende", "Legend"],
    ["cachée", "hidden"], ["${} cachées", "${} hidden"], ["COPIE", "COPY"], ["JETON", "TOKEN"],
    ["· coût ${}", "· cost ${}"], ["· might ${}", "· might ${}"], ["· bonus +${}", "· bonus +${}"],
    ["Ce coup n'est pas possible.", "This move isn't possible."], ["Cette unité n'est pas une cible possible.", "This unit isn't a possible target."],
    ["à toi", "yours"], ["Jouer", "Play"], ["Jouer → ${}", "Play → ${}"],
    ["Déplacer… (puis choisir la destination)", "Move… (then choose the destination)"], ["Lire la carte", "Read the card"], ["Lire", "Read"],
    ["Remettre ${} carte${}", "Put back ${} card${}"], ["Garder ma main", "Keep my hand"],
    ["Mulligan : touche jusqu'à 2 cartes à remettre.", "Mulligan: tap up to 2 cards to put back."],
    ["Utiliser cet effet ?", "Use this effect?"], ["Oui", "Yes"], ["Non", "No"],
    ["Touche une carte ou une zone en surbrillance.", "Tap a highlighted card or zone."], ["Valider", "Confirm"],
    ["Depuis la défausse", "From the trash"], ["Depuis la main", "From the hand"],
    ["Accelerate (+1 énergie +1 ${}, arrive prête)", "Accelerate (+1 energy +1 ${}, enters ready)"], ["Normal", "Normal"],
    ["Victoire !", "Victory!"], ["Victoire", "Victory"], ["Défaite", "Defeat"], ["Partie nulle", "Draw"],
    ["Manche suivante", "Next game"], ["Nouveau match", "New match"], ["Rejouer la même donne", "Replay the same deal"],
    ["Manche ${} terminée · Match : ${}", "Game ${} over · Match: ${}"],
    ["Allié à déplacer", "Ally to move"], ["Ennemi", "Enemy"], ["Destination", "Destination"],
    ["Touche l'ennemi qui prend les dégâts.", "Tap the enemy that takes the damage."],
    ["Touche l'allié à déplacer.", "Tap the ally to move."], ["Touche la zone où il va.", "Tap the zone it goes to."],
    ["Tout est choisi.", "Everything is chosen."], ["Annuler", "Cancel"],
    ["Cibles ${}/${} : ${}", "Targets ${}/${}: ${}"], ["touche une carte en surbrillance", "tap a highlighted card"],
    ["valide", "confirm"], ["(retouche une cible pour l'enlever)", "(tap a target again to remove it)"],
    ["touche une carte en surbrillance (retouche une cible pour l'enlever)", "tap a highlighted card (tap a target again to remove it)"],
    ["valide (retouche une cible pour l'enlever)", "confirm (tap a target again to remove it)"],
    ["Valider la cible", "Confirm the target"], ["touche la destination", "tap the destination"],
    ["${} unité${} → ${}", "${} unit${} → ${}"], ["Ces unités ne peuvent pas partir ensemble vers cette zone.", "These units can't go to this zone together."],
    ["⚔ Lancer le combat", "⚔ Start the combat"], ["Lancer le déplacement", "Start the move"],
    ["${}. Touche d'autres unités pour les ajouter.", "${}. Tap other units to add them."], ["comment le jouer ?", "how do you play it?"],
    ["Pas d'ennemi", "No enemy"], ["meilleur", "best"], ["(ton coup)", "(your move)"],
    // ---------------------------------------------------------------- conseil, défausse, dessus connu
    ["Conseil de l'IA", "AI hint"], ["Calcul…", "Thinking…"], ["Conseil impossible : ${}", "Hint impossible: ${}"],
    ["Rien à conseiller ici.", "Nothing to suggest here."],
    // ---------------------------------------------------------------- erreur de l'IA marquée par le joueur (2026-10-28)
    ["⚑ Erreur de l'IA", "⚑ AI mistake"], ["Erreur de l'IA", "AI mistake"],
    ["Marque le dernier coup de l'IA comme une erreur, pour améliorer l'IA", "Mark the AI's last move as a mistake, to improve the AI"],
    ["Mauvaise cible", "Wrong target"], ["Mauvais moment", "Wrong timing"], ["Coup inutile", "Useless move"], ["Autre erreur", "Other mistake"],
    ["Le coup est gardé avec la partie, pour tester l'IA corrigée sur cette position.", "The move is saved with the game, to test the fixed AI on this position."],
    ["Marque impossible : ${}", "Cannot mark: ${}"], ["Pas de coup de l'IA à marquer", "No AI move to mark"],
    ["Ce coup est déjà marqué", "This move is already marked"], ["Coup de l'IA marqué ✓", "AI move marked ✓"],
    ["Quelle erreur ? (ex. il devait attaquer)", "What mistake? (e.g. it should have attacked)"], ["Quelle erreur ?", "What mistake?"],
    ["Écris quelle erreur", "Write what the mistake is"],
    ["Simulation sur 2 tours. Écart avec le meilleur coup ; 7 vaut environ un point, sous 2 c'est du bruit. Touche une ligne pour la jouer.",
     "Simulation over 2 turns. Gap to the best move; 7 is worth about one point, under 2 it's noise. Tap a line to play it."],
    ["Reprise impossible : ${}", "Undo impossible: ${}"],
    ["Dessus connu de ton deck (${})", "Known top of your deck (${})"],
    ["Dans l'ordre : la première sera piochée en premier. Vu par Predict ou Vision ; disparaît dès que la carte quitte le dessus.",
     "In order: the first one will be drawn first. Seen with Predict or Vision; disappears as soon as the card leaves the top."],
    ["Rien de connu pour l'instant.", "Nothing known yet."],
    ["La plus récente en premier. Touche une carte pour la lire.", "Most recent first. Tap a card to read it."],
    // ---------------------------------------------------------------- nouvelle partie contre l'IA, match
    ["(pas encore jouable)", "(not playable yet)"], ["aucun pour l'instant", "none yet"], ["Mes decks", "My decks"],
    ["Decks de référence", "Reference decks"], ["Listes de tournoi", "Tournament lists"],
    ["le plan Hook tempo", "the Hook tempo plan"], ["le plan Gorica", "the Gorica plan"],
    ["l'IA générique (sans plan propre à ce deck)", "the generic AI (no plan specific to this deck)"],
    ["bientôt", "soon"], ["Au hasard", "Random"], ["Légende au hasard pour l'IA", "Random legend for the AI"], ["Hasard", "Random"],
    ["toi", "you"], ["l'IA", "the AI"], ["Ton deck (variantes de la légende)", "Your deck (variants of the legend)"],
    ["Deck de l'IA (variantes de la légende)", "AI deck (variants of the legend)"], ["Niveau de l'IA", "AI level"], ["Niv.", "Lvl"],
    ["Fort", "Strong"], ["Choisir pour ${}", "Choose for ${}"], ["IA", "AI"], ["TOI", "YOU"], ["Légendes", "Legends"],
    ["Mode de match", "Match mode"], ["Mode", "Mode"], ["une manche", "one game"], ["2 manches gagnantes, sideboard", "best of 3, sideboard"],
    ["Contre l'IA", "Against the AI"],
    ["Touche ton panneau ou celui de l'IA, puis une légende. Chaque légende arrive avec son deck par défaut (meilleure liste de tournoi jouable, sideboard compris).",
     "Tap your panel or the AI's, then a legend. Each legend comes with its default deck (best playable tournament list, sideboard included)."],
    ["Commencer", "Start"], ["Plus d'options", "More options"], ["Ton battlefield", "Your battlefield"], ["Qui commence", "Who goes first"],
    ["Pile ou face", "Coin toss"], ["Moi", "Me"], ["L'IA", "The AI"],
    ["Donne (vide = au hasard, même numéro = même partie)", "Deal (empty = random, same number = same game)"], ["au hasard", "random"],
    ["Créer ou modifier mes decks", "Create or edit my decks"],
    ["Tirage au sort, puis le perdant de chaque manche choisit qui commence (règles 407.1, 407.4). Tu choisis ton battlefield à chaque manche ; ceux d'une manche gagnée sont retirés (486.5). Sideboard entre les manches, 1 pour 1 (403.4). L'IA ne sideboarde pas encore : elle garde son deck de la manche 1.",
     "Coin toss, then the loser of each game chooses who goes first (rules 407.1, 407.4). You choose your battlefield each game; those of a won game are removed (486.5). Sideboarding between games, 1 for 1 (403.4). The AI doesn't sideboard yet: it keeps its game 1 deck."],
    ["Une manche, avec le même deck qu'en BO3. Battlefields tirés au hasard pour toi et l'IA (règle 485.5), sauf si tu imposes le tien (plus d'options).",
     "One game, with the same deck as in BO3. Battlefields drawn at random for you and the AI (rule 485.5), unless you pick yours (more options)."],
    ["Commencer le match", "Start the match"], ["Au hasard (règle 485.5)", "Random (rule 485.5)"],
    ["Choix du plan Gorica (Sigil si LeBlanc prend Windswept, sinon Void Gate)", "Gorica plan's choice (Sigil if LeBlanc takes Windswept, otherwise Void Gate)"],
    ["deck par défaut de la légende tirée", "default deck of the drawn legend"], ["par défaut · ${}", "default · ${}"],
    ["sans sideboard", "no sideboard"], ["⚠ bannies depuis : ${}", "⚠ banned since: ${}"],
    ["L'IA tire une légende au hasard parmi celles qui ont un deck jouable, avec son deck par défaut. Elle ne voit pas ta main.",
     "The AI draws a random legend among those with a playable deck, with its default deck. It doesn't see your hand."],
    ["L'IA joue ${} avec l'IA « gagner à tout prix ». Elle ne voit pas ta main.", "The AI plays ${} with the \"win at all costs\" AI. It doesn't see your hand."],
    ["Choisis ta légende, celle de l'IA, puis Commencer.", "Choose your legend, the AI's, then Start."],
    ["toi ${} – ${} IA", "you ${} – ${} AI"], ["toi ${} – ${} ${}", "you ${} – ${} ${}"], ["Erreur du moteur : ${}", "Engine error: ${}"],
    ["Tu gagnes le tirage au sort.", "You win the coin toss."], ["L'IA gagne le tirage au sort.", "The AI wins the coin toss."],
    ["${} gagne le tirage au sort.", "${} wins the coin toss."],
    ["Règles 407.1-407.2 : le gagnant du tirage choisit de jouer en premier ou en second.", "Rules 407.1-407.2: the winner of the toss chooses to play first or second."],
    ["Tu as perdu la manche précédente : à toi de choisir.", "You lost the previous game: your choice."],
    ["Règle 407.4 : le perdant de la manche précédente choisit.", "Rule 407.4: the loser of the previous game chooses."],
    ["L'IA a perdu la manche précédente : elle choisit.", "The AI lost the previous game: it chooses."],
    ["${} a perdu la manche précédente : à lui de choisir.", "${} lost the previous game: their choice."],
    ["Manche précédente nulle : même premier joueur.", "Previous game drawn: same first player."],
    ["Règle 407.4 : après une nulle, le joueur qui a commencé recommence.", "Rule 407.4: after a draw, the player who went first goes first again."],
    ["Jouer en premier ou en second ?", "Play first or second?"], ["Jouer en premier", "Play first"], ["Jouer en second", "Play second"],
    ["L'IA choisit de jouer en premier.", "The AI chooses to play first."], ["L'IA choisit de jouer en second : tu commences.", "The AI chooses to play second: you go first."],
    ["L'IA commence.", "The AI goes first."], ["tu commences", "you go first"],
    ["Déjà joué${} lors d'une manche gagnée par un joueur, retiré${} pour le reste du match (règle 486.5 ; après une nulle ils resservent, 486.5.a) : ${}.",
     "Already played in a game won by a player, removed for the rest of the match (rule 486.5; after a draw they can be used again, 486.5.a): ${3}."],
    ["Déjà joué${} lors d'une manche gagnée, retiré${} pour le reste du match (règle 486.5) : ${}.",
     "Already played in a won game, removed for the rest of the match (rule 486.5): ${3}."],
    ["Choisis ton battlefield", "Choose your battlefield"],
    ["Choix simultanés : l'IA a choisi le sien sans voir le tien ; les deux sont révélés au lancement (règle 486.5).",
     "Simultaneous choices: the AI chose its own without seeing yours; both are revealed at the start (rule 486.5)."],
    ["Manche ${} · BO3", "Game ${} · BO3"], ["Match : ${}", "Match: ${}"], ["Battlefield", "Battlefield"],
    ["Lancer la manche ${}", "Start game ${}"],
    ["Garder", "Keep"], ["Ne pas faire entrer", "Don't bring in"], ["Chosen Champion (règle 601.1.c.4)", "Chosen Champion (rule 601.1.c.4)"],
    ["Sortent", "Out"], ["· entrent", "· in"], [": il en faut autant de chaque côté (403.4)", ": there must be as many on each side (403.4)"],
    ["Sideboard avant la manche ${}", "Sideboard before game ${}"],
    ["Échanges 1 pour 1 entre ton Main Deck et ton sideboard (règle 403.4) ; runes, légende et battlefields ne changent pas (403.4.b). Seul ce match est concerné : ton deck enregistré ne change pas (403.8).",
     "1-for-1 swaps between your Main Deck and your sideboard (rule 403.4); runes, legend and battlefields don't change (403.4.b). Only this match is affected: your saved deck doesn't change (403.8)."],
    ["Main Deck : touche + pour sortir", "Main Deck: tap + to take out"], ["Sideboard : touche + pour faire entrer", "Sideboard: tap + to bring in"],
    ["Garder le deck", "Keep the deck"], ["Valider les échanges", "Confirm the swaps"], ["deck gardé", "deck kept"],
    ["${} échange${}", "${} swap${}"], ["Deck modifié pour ce match : ${}.", "Deck changed for this match: ${}."],
    ["Donne ${} · ${} contre ${} · ${} · ${} / ${} · IA ${}", "Deal ${} · ${} vs ${} · ${} · ${} / ${} · ${} AI"],
    ["Battlefields révélés : toi ${} · IA ${}", "Battlefields revealed: you ${} · AI ${}"],
    ["Battlefields révélés : toi ${} · ${}", "Battlefields revealed: you ${} · ${}"],
    ["gagné", "won"], ["Match ${} · ${}", "Match ${} · ${}"], ["Manche ${} · ${}", "Game ${} · ${}"],
    ["Victoire du match ${}", "Match won ${}"], ["Défaite du match ${}", "Match lost ${}"], ["Match nul ${}", "Match drawn ${}"],
    // ---------------------------------------------------------------- sons
    ["coupés", "muted"], ["Son activé", "Sound on"], ["Son coupé", "Sound off"], ["Volume", "Volume"], ["Volume des sons", "Sound volume"],
    ["Ambiance", "Theme"], ["Cartes", "Cards"], ["Cinéma", "Cinema"], ["Feutré", "Muffled"], ["Bois", "Wood"], ["Verre", "Glass"],
    ["Studio", "Studio"], ["Zen", "Zen"],
    // ---------------------------------------------------------------- erreurs, chargement
    ["Le moteur n'a pas pu se charger : ${}. Recharge la page ; si l'erreur revient, dis-le dans le fil.",
     "The engine couldn't load: ${}. Reload the page; if the error comes back, say so in the thread."],
    ["Le moteur n'a pas pu se charger : ${}. Recharge la page.", "The engine couldn't load: ${}. Reload the page."],
    ["erreur de vérification : ${}", "check error: ${}"], ["Le moteur se charge encore…", "The engine is still loading…"],
    // ---------------------------------------------------------------- éditeur de deck
    ["Mon deck", "My deck"], ["Deck gardé sur cet appareil (table indisponible).", "Deck kept on this device (table unavailable)."],
    ["Deck enregistré.", "Deck saved."], ["Deck gardé sur cet appareil seulement.", "Deck kept on this device only."],
    ["Supprimer le deck « ${} » ?", "Delete the deck \"${}\"?"], ["Deck supprimé.", "Deck deleted."],
    ["Le sideboard ne prend que des cartes du deck principal : pas de légende, rune ni battlefield (règle 601.1.c).",
     "The sideboard only takes main deck cards: no legend, rune or battlefield (rule 601.1.c)."],
    ["Sideboard plein : 10 cartes au plus (règle 601.1.c.1).", "Sideboard full: 10 cards at most (rule 601.1.c.1)."],
    ["3 exemplaires au plus, sideboard compris (règle 601.1.c.3).", "3 copies at most, sideboard included (rule 601.1.c.3)."],
    ["${} n'est pas encore modélisée : tu peux préparer le deck, mais pas encore le jouer.", "${} isn't modelled yet: you can build the deck, but not play it yet."],
    ["Déjà 3 battlefields : retire-en un d'abord.", "Already 3 battlefields: remove one first."], ["Déjà 12 runes.", "Already 12 runes."],
    ["3 exemplaires au plus.", "3 copies at most."], ["${} (pas encore modélisée)", "${} (not modelled yet)"], ["à modéliser", "to be modelled"],
    ["Passer ${} au sideboard", "Move ${} to the sideboard"], ["Passer un exemplaire au sideboard", "Move a copy to the sideboard"],
    ["Retirer ${}", "Remove ${}"], ["Ajouter ${}", "Add ${}"], ["Unités", "Units"], ["Sorts", "Spells"], ["Équipements", "Gear"],
    ["Retirer la légende", "Remove the legend"], ["Touche une légende dans la liste.", "Tap a legend in the list."],
    ["Champion choisi", "Chosen champion"], ["Touche une unité Champion de ta légende.", "Tap a Champion unit of your legend."],
    ["Choisis d'abord la légende.", "Choose the legend first."], ["Deck principal ${}/40", "Main deck ${}/40"], ["Deck principal", "Main deck"],
    ["En faire le champion choisi", "Make it the chosen champion"], ["Passer ${} au deck principal", "Move ${} to the main deck"],
    ["Passer un exemplaire au deck principal", "Move a copy to the main deck"], ["Retirer ${} du sideboard", "Remove ${} from the sideboard"],
    ["Ajouter ${} au sideboard", "Add ${} to the sideboard"],
    ["Vide. Choisis « Ajouter au : Sideboard » au-dessus de la liste, ou touche ⇄ sur une carte du deck.",
     "Empty. Choose \"Add to: Sideboard\" above the list, or tap ⇄ on a deck card."],
    ["Entre les manches d'un BO3 : échanges 1 pour 1 avec le deck (règle 403.4). 10 cartes au plus, 3 exemplaires deck + sideboard (601.1.c).",
     "Between the games of a BO3: 1-for-1 swaps with the deck (rule 403.4). 10 cards at most, 3 copies deck + sideboard (601.1.c)."],
    ["Retirer une rune ${}", "Remove a ${} rune"], ["Ajouter une rune ${}", "Add a ${} rune"],
    ["Touche 3 battlefields dans la liste.", "Tap 3 battlefields in the list."],
    ["Deck jouable ✓", "Playable deck ✓"], ["Pas encore jouable", "Not playable yet"],
    ["au", "to the"], ["sideboard", "sideboard"], ["(légendes, runes et battlefields vont toujours au deck)", "(legends, runes and battlefields always go to the deck)"],
    ["au deck", "to the deck"], ["Afficher ${} de plus", "Show ${} more"], ["Supprimer", "Delete"],
    ["Ouvrir un deck", "Open a deck"], ["Ouvrir…", "Open…"], ["Copier un deck de référence", "Copy a reference deck"],
    ["Chercher (nom, texte, tag, mot-clé)", "Search (name, text, tag, keyword)"], ["Ajouter au", "Add to"], ["Ajouter au :", "Add to:"],
    ["Nom du deck", "Deck name"], ["← Menu", "← Menu"], ["Nouveau", "New"], ["Importer", "Import"], ["Exporter", "Export"],
    ["Enregistrer", "Save"], ["${} cartes jouables sur ${}", "${} playable cards out of ${}"], ["Tous les types", "All types"],
    ["Toutes les extensions", "All sets"], ["Domaines de ma légende", "My legend's domains"], ["Tous les domaines", "All domains"],
    ["${} carte${}. Touche une carte pour l'ajouter ${} ; les cartes grisées ne sont pas encore modélisées.",
     "${} card${}. Tap a card to add it ${}; greyed-out cards aren't modelled yet."],
    ["Jouer ce deck", "Play this deck"], ["Exporter « ${} »", "Export \"${}\""], ["Copier", "Copy"], ["Copié.", "Copied."],
    ["Importer un deck", "Import a deck"],
    ["Colle une liste, une ligne par carte : « 3 Nom de la carte ». Les runes s'écrivent « 6 Fury Rune ». La légende, les battlefields et le champion sont reconnus tout seuls.",
     "Paste a list, one line per card: \"3 Card Name\". Runes are written \"6 Fury Rune\". The legend, the battlefields and the champion are recognised automatically."],
    ["Cartes inconnues : ${}", "Unknown cards: ${}"], ["Deck importé.", "Deck imported."], ["Deck importé", "Imported deck"],
    ["Légende :", "Legend:"], ["Deck principal :", "Main deck:"],
    // ---------------------------------------------------------------- duel entre amis
    ["ton ami", "your friend"], ["rôle : ${}", "role: ${}"], ["Déconnecté · toucher pour revenir", "Disconnected · tap to come back"],
    ["${} déconnecté", "${} disconnected"], ["En ligne · room ${}", "Online · room ${}"],
    ["Room ${} ouverte", "Room ${} open"], ["Code de la room", "Room code"], ["Lien de la room", "Room link"],
    ["Copier le lien", "Copy the link"], ["Fermer la room", "Close the room"],
    ["Donne ce code (ou le lien) à ton ami : la partie commence dès qu'il rejoint. Garde cette page ouverte.",
     "Give this code (or the link) to your friend: the game starts as soon as they join. Keep this page open."],
    ["Format du match", "Match format"], ["Jouer avec un ami", "Play with a friend"], ["Ton pseudo", "Your nickname"], ["ex. Huy", "e.g. Huy"],
    ["Ton deck", "Your deck"], ["Journal de connexion", "Connection log"], ["Copier le diagnostic", "Copy the diagnostics"],
    ["Créer une room", "Create a room"], ["Rejoindre une room", "Join a room"], ["Rejoindre", "Join"], ["Code de la room (ou lien)", "Room code (or link)"],
    ["Connexion directe entre vos deux navigateurs (WebRTC) ; le serveur public PeerJS ne sert qu'à vous mettre en relation. Chaque navigateur fait tourner toute la partie : la main adverse n'est pas affichée, mais reste lisible dans la console du navigateur. Les deux pages doivent être la même version de la table. Règles du match : celles de « Nouvelle partie » (BO1 485, BO3 486, sideboard 403).",
     "Direct connection between your two browsers (WebRTC); the public PeerJS server only puts you in touch. Each browser runs the whole game: the opponent's hand isn't shown, but stays readable in the browser console. Both pages must be the same version of the table. Match rules: those of \"New game\" (BO1 485, BO3 486, sideboard 403)."],
    ["Diagnostic copié : colle-le à Claude.", "Diagnostics copied: paste them to Claude."],
    ["Copie refusée : le texte est sélectionné, copie-le à la main.", "Copy refused: the text is selected, copy it by hand."],
    ["Le code fait 6 caractères.", "The code is 6 characters long."], ["Lien copié : ${}", "Link copied: ${}"],
    ["Copie impossible : sélectionne le lien et copie-le.", "Can't copy: select the link and copy it."], ["Choisis un deck.", "Choose a deck."],
    ["Deck pas jouable : ${}", "Deck not playable: ${}"], ["PeerJS n'a pas pu se charger (vendor/peerjs.min.js).", "PeerJS couldn't load (vendor/peerjs.min.js)."],
    ["Ouverture de la room…", "Opening the room…"], ["Room ouverte : en attente de ton ami…", "Room open: waiting for your friend…"],
    ["Ton ami se connecte…", "Your friend is connecting…"], ["Erreur de connexion (${}). Vérifie ta connexion et recommence.", "Connection error (${}). Check your connection and try again."],
    ["Erreur de connexion (${}).", "Connection error (${})."], ["Cette room est déjà occupée.", "This room is already taken."],
    ["${} est revenu : reprise de la partie.", "${} is back: the game resumes."],
    ["Versions différentes de la table (toi ${}, hôte ${}) : rechargez tous les deux la page.", "Different versions of the table (you ${}, host ${}): both of you reload the page."],
    ["Deck refusé : ${}", "Deck refused: ${}"], ["Invité", "Guest"], ["Hôte", "Host"], ["hôte", "host"], ["invité", "guest"],
    ["Connexion à la room ${}…", "Connecting to room ${}…"], ["Connecté : l'hôte prépare le match…", "Connected: the host is preparing the match…"],
    ["L'hôte ne répond pas. Ouvre « Journal de connexion », touche « Copier le diagnostic » et envoie-le à Claude.",
     "The host isn't answering. Open \"Connection log\", tap \"Copy the diagnostics\" and send them to Claude."],
    ["Room ${} introuvable : vérifie le code ; l'hôte doit garder sa page ouverte.", "Room ${} not found: check the code; the host must keep their page open."],
    ["Reconnexion à la room ${}…", "Reconnecting to room ${}…"],
    ["${} s'est déconnecté : la room reste ouverte, il peut recharger sa page pour revenir.", "${} disconnected: the room stays open, they can reload their page to come back."],
    ["Connexion perdue avec l'hôte. Touche « Déconnecté » en haut pour revenir.", "Connection to the host lost. Tap \"Disconnected\" at the top to come back."],
    ["Erreur du duel : ${}", "Duel error: ${}"], ["Refusé par l'hôte.", "Refused by the host."], ["${} a quitté le duel.", "${} left the duel."],
    ["Désynchronisation détectée : l'invité doit recharger sa page.", "Desync detected: the guest must reload their page."],
    ["${} reprend son dernier coup.", "${} takes back their last move."],
    ["Tirés au hasard (règle 485.5)", "Drawn at random (rule 485.5)"], ["Toi : ${} · ${} : ${}", "You: ${} · ${}: ${}"], ["Toi :", "You:"],
    ["Ton battlefield : ${}", "Your battlefield: ${}"], ["Valider mon battlefield", "Confirm my battlefield"],
    ["${} a choisi son battlefield : il sera révélé quand vous aurez choisi tous les deux (règle 486.5).", "${} chose their battlefield: it will be revealed once you have both chosen (rule 486.5)."],
    ["${} choisit son battlefield…", "${} is choosing their battlefield…"], ["${} choisit qui commence…", "${} is choosing who goes first…"],
    ["${} fait son sideboard…", "${} is sideboarding…"], ["Tout est choisi : la manche démarre…", "Everything is chosen: the game is starting…"],
    ["La manche démarre dès que vous avez tous les deux choisi.", "The game starts as soon as you have both chosen."],
    ["Manche ${} · ${} · duel", "Game ${} · ${} · duel"], ["Quitter le duel", "Leave the duel"], ["Quitter le duel ? Ton ami sera prévenu.", "Leave the duel? Your friend will be told."],
    ["Duel · room ${} · ${} · ${} contre ${} · ${} · ${} / ${}", "Duel · room ${} · ${} · ${} vs ${} · ${} · ${} / ${}"],
    ["Reconnecté.", "Reconnected."], ["Partie reprise : ${} coups rejoués.", "Game resumed: ${} moves replayed."], ["Retour au menu", "Back to the menu"],
    ["Retour au duel", "Back to the duel"], ["Duel en ligne avec un code de room.", "Online duel with a room code."],
    ["Room ${} contre ${}.", "Room ${} against ${}."], ["BO1", "BO1"], ["BO3", "BO3"],
    // ---------------------------------------------------------------- replay (« Mes parties » → « Revoir »)
    ["Revoir la partie", "Review the game"], ["Revoir", "Review"], ["Le moteur rejoue la partie coup par coup…", "The engine replays the game move by move…"],
    ["L'IA recalcule ses coups : de quelques secondes à une minute.", "The AI recomputes its moves: from a few seconds to a minute."],
    ["Rejeu impossible : ${}", "Replay impossible: ${}"],
    ["moteur différent de celui de la partie : le rejeu peut s'écarter", "engine different from the game's: the replay may diverge"],
    ["le rejeu s'arrête à ta décision n° ${} (moteur changé depuis)", "the replay stops at your decision no. ${} (engine changed since)"],
    ["fin différente de la partie enregistrée (moteur changé depuis)", "ending different from the saved game (engine changed since)"],
    ["Replay · donne ${} · ${} contre ${} · ${}", "Replay · deal ${} · ${} vs ${} · ${}"], ["· BO3 manche ${}", "· BO3 game ${}"],
    ["tour ${} · étape ${}/${}", "turn ${} · step ${}/${}"], ["↺ Revoir du début", "↺ Review from the start"],
    ["Ton coup n'est pas parmi les ${} meilleurs de l'IA.", "Your move isn't among the AI's ${} best."],
    ["Simulation sur 2 tours. Écart avec le meilleur coup ; 7 vaut environ un point, sous 2 c'est du bruit.",
     "Simulation over 2 turns. Gap to the best move; 7 is worth about one point, under 2 it's noise."],
    ["Rien à conseiller ici (un seul coup possible).", "Nothing to suggest here (only one possible move)."],
    ["▴ Afficher le détail", "▴ Show the details"], ["▾ Réduire pour voir le plateau", "▾ Collapse to see the board"],
    ["Analyse…", "Analysing…"], ["Analyser ce coup", "Analyse this move"], ["Réponse à une question : pas de conseil.", "Answer to a question: no hint."],
    ["Ici, ${}", "Here, ${}"], ["Navigation du replay", "Replay navigation"], ["Début", "Start"], ["Ta décision précédente", "Your previous decision"],
    ["Étape précédente", "Previous step"], ["Étape suivante", "Next step"], ["Ta décision suivante", "Your next decision"],
    ["Étape du replay", "Replay step"], ["Quitter le replay", "Leave the replay"], ["Quitter", "Leave"],
    ["Choisis une partie à revoir, ou commence une nouvelle partie.", "Choose a game to review, or start a new game."],
    // ---------------------------------------------------------------- relevés du balayage complet (2026-10-17)
    ["phase principale", "main phase"],
    ["jouable", "playable"],
    ["Cibler", "Target"],
    ["${} : cibler", "${}: target"],
    ["Avec Repeat", "With Repeat"],
    ["Sans Repeat", "Without Repeat"],
    ["Sans coût supplémentaire", "Without the additional cost"],
    ["Cacher → ${}", "Hide → ${}"],
    ["Enregistrement ✓", "Saved ✓"],
    ["Stockage plein", "Storage full"],
    ["Ton battlefield :", "Your battlefield:"],
    ["Choisi ✓", "Chosen ✓"],
    ["${} (pas encore jouable)", "${} (not playable yet)"],
    ["Vide.", "Empty."], ["${} échange${}, champion : ${}", "${1} swap${2}, champion: ${3}"],
    ["${} · ⚠ bannies depuis : ${}", "${} · ⚠ banned since: ${}"],
    ["La plus récente en premier. Touche une carte pour la lire.", "Most recent first. Tap a card to read it."],
    [", champion : ${}", ", champion: ${}"],
    // ---------------------------------------------------------------- tutoriel et lexique
    ["Priorité", "Priority"],
    ["Le droit d'agir maintenant ; un seul joueur l'a à la fois. Quand une carte est jouée, la priorité passe à l'adversaire : il peut répondre avec une Reaction, ou passer. Quand tous les joueurs ont passé à la suite, la carte se résout.", "The right to act now; only one player has it at a time. When a card is played, priority goes to the opponent: they can respond with a Reaction, or pass. When all players have passed in a row, the card resolves."],
    ["Pendant un showdown, le joueur qui a le focus peut jouer une carte Action ou Reaction, ou passer. Après chaque carte jouée, le focus va à l'autre joueur. Quand tous ont passé à la suite, le showdown se termine.", "During a showdown, the player with focus can play an Action or Reaction card, or pass. After each card played, focus goes to the other player. When everyone has passed in a row, the showdown ends."],
    ["Dire « je n'ai rien à ajouter ». Ce n'est pas abandonner ni perdre son tour : le jeu attend que chacun passe pour avancer. Sans carte Action ou Reaction utile, passer est le bon coup.", "Saying \"I have nothing to add\". It isn't giving up or losing your turn: the game waits for everyone to pass before moving on. Without a useful Action or Reaction card, passing is the right move."],
    ["Le showdown : à qui de parler ?", "The showdown: whose turn to speak?"],
    ["Déplace une unité vers un champ : le jeu ouvrira un showdown.", "Move a unit to a battlefield: the game will open a showdown."],
    ["Ton unité arrive sur un champ : avant que le champ change de mains, un showdown s'ouvre. Les joueurs parlent chacun leur tour ; celui dont c'est le tour a le focus, ici toi. Avec le focus, tu peux jouer une carte Action ou Reaction (cadre vert), ou passer. Passer veut dire « rien à ajouter » : ce n'est pas perdre ton tour. Quand tout le monde a passé à la suite, le showdown se termine.", "Your unit arrives on a battlefield: before the battlefield changes hands, a showdown opens. Players speak one at a time; whoever's turn it is to speak has focus, here you. With focus, you can play an Action or Reaction card (green frame), or pass. Passing means \"nothing to add\": it isn't losing your turn. When everyone has passed in a row, the showdown ends."],
    ["Action et Reaction", "Action and Reaction"],
    ["Seules les cartes marquées Action ou Reaction se jouent pendant un showdown. Punch First est une Action : +5 Might à une unité, de quoi gagner un combat. Sans ennemi sur ce champ, garde-la pour un combat. Reaction va plus loin : elle se joue à tout moment, même en réponse à une carte adverse, avant qu'elle fasse effet.", "Only cards marked Action or Reaction can be played during a showdown. Punch First is an Action: +5 Might to a unit, enough to win a combat. With no enemy on this battlefield, keep it for a combat. Reaction goes further: it can be played at any time, even in response to an opponent's card, before it takes effect."],
    ["Touche « Passer » si tu n'as rien à ajouter. Si Darius passe aussi, le showdown se termine et ton unité conquiert le champ : +1 point.", "Tap \"Pass\" if you have nothing to add. If Darius passes too, the showdown ends and your unit conquers the battlefield: +1 point."],
    ["Tu as la priorité", "You have priority"],
    ["Une carte ou un effet attend sur la chaîne (surlignée) : il n'a pas encore fait effet. Tu as la priorité, c'est-à-dire le droit de répondre maintenant, avec une carte Reaction ; ta réponse se résoudrait en premier. Rien à répondre ? Touche « Passer » : quand chacun a passé, l'effet se résout.", "A card or effect is waiting on the chain (highlighted): it hasn't taken effect yet. You have priority, that is the right to respond now, with a Reaction card; your response would resolve first. Nothing to respond with? Tap \"Pass\": once everyone has passed, the effect resolves."],
    ["À toi de parler", "Your turn to speak"],
    ["Un showdown est en cours et tu as le focus : joue une carte Action ou Reaction, ou touche « Passer » si tu n'as rien d'utile.", "A showdown is under way and you have focus: play an Action or Reaction card, or tap \"Pass\" if you have nothing useful."],
    ["Jouer pendant l'adversaire : Action et Reaction", "Playing during the opponent's time: Action and Reaction"],
    ["D'habitude, tu ne joues tes cartes que pendant ton tour, quand rien d'autre ne se passe. Deux mots-clés permettent plus. Action : la carte se joue aussi pendant un showdown, la pause avant qu'un champ change de mains ou qu'un combat fasse ses dégâts, même sur le tour adverse. Reaction : la carte se joue à tout moment, même en réponse à une autre carte, avant qu'elle fasse effet.", "Usually, you only play your cards on your turn, when nothing else is happening. Two keywords allow more. Action: the card can also be played during a showdown, the pause before a battlefield changes hands or a combat deals its damage, even on the opponent's turn. Reaction: the card can be played at any time, even in response to another card, before it takes effect."],
    ["Chacun son tour de parole", "One speaker at a time"],
    ["Un seul joueur peut agir à la fois.", "Only one player can act at a time."],
    ["Quand une carte est jouée, l'adversaire reçoit la priorité : il peut répondre avec une Reaction, ou passer.", "When a card is played, the opponent receives priority: they can respond with a Reaction, or pass."],
    ["Pendant un showdown, celui qui a le focus joue une Action ou une Reaction, ou passe ; puis le focus va à l'autre.", "During a showdown, whoever has focus plays an Action or a Reaction, or passes; then focus goes to the other player."],
    ["Quand tout le monde a passé à la suite, le jeu avance : la carte se résout, ou le showdown se termine.", "When everyone has passed in a row, the game moves on: the card resolves, or the showdown ends."],
    ["Passer n'est pas perdre son tour : c'est dire « je n'ai rien à ajouter ». Sans carte utile, c'est le bon coup.", "Passing isn't losing your turn: it's saying \"I have nothing to add\". Without a useful card, it's the right move."],
    ["Retour", "Back"], ["Passer l'intro", "Skip the intro"], ["Jouer la partie guidée", "Play the guided game"],
    ["Tutoriel · ${} contre ${} (IA débutante)", "Tutorial · ${} vs ${} (beginner AI)"], ["Défi réussi : ${} !", "Challenge complete: ${}!"],
    ["Tutoriel réussi", "Tutorial complete"], ["Tutoriel réussi !", "Tutorial complete!"],
    ["Tu connais les bases. Finis la partie jusqu'à 8 points, ou lance une vraie partie depuis le menu.", "You know the basics. Finish the game up to 8 points, or start a real game from the menu."],
    ["Défi en cours : ${}.", "Current challenge: ${}."], ["Défi ${} · ${}", "Challenge ${} · ${}"],
    ["Victoire ${}-${} !", "Victory ${}-${}!"], ["Défaite ${}-${}", "Defeat ${}-${}"], ["Partie terminée", "Game over"],
    ["Tu peux recommencer le tutoriel : même donne, même main de départ.", "You can restart the tutorial: same deal, same starting hand."],
    ["Continuer la partie", "Continue the game"], ["${} défi${} réussi${} sur ${}.", "${1} challenge${2} complete out of ${4}."],
    ["Recommencer le tuto", "Restart the tutorial"], ["Partie contre l'IA", "Game against the AI"],
    ["La force d'une unité. En combat, chaque camp inflige autant de dégâts que la Might totale de ses unités ; une unité meurt quand elle a reçu des dégâts au moins égaux à sa Might.", "A unit's strength. In combat, each side deals as much damage as the total Might of its units; a unit dies when it has taken damage at least equal to its Might."],
    ["Énergie", "Energy"],
    ["Ce qui paie le coût d'une carte (pastille dorée en haut à gauche). Chaque rune épuisée donne 1 énergie. L'énergie non dépensée est perdue à la fin du tour.", "What pays a card's cost (gold badge at the top left). Each exhausted rune gives 1 energy. Unspent energy is lost at the end of the turn."],
    ["Deuxième partie du coût de certaines cartes : le « +2 » de « 1+2 ». Elle se paie en recyclant des runes de la couleur (domaine) de la carte.", "Second part of some cards' cost: the \"+2\" in \"1+2\". It is paid by recycling runes of the card's colour (domain)."],
    ["Ta source d'énergie. Tu en poses 2 par tour depuis ta réserve. Une rune s'épuise pour donner 1 énergie, ou se recycle pour donner 1 puissance de sa couleur.", "Your source of energy. You put down 2 per turn from your rune deck. A rune exhausts to give 1 energy, or is recycled to give 1 power of its colour."],
    ["Épuisé", "Exhausted"],
    ["Une carte tournée sur le côté est épuisée : elle a déjà servi. Une unité épuisée ne peut pas se déplacer. Tout ce que tu contrôles se redresse au début de ton tour.", "A card turned sideways is exhausted: it has already been used. An exhausted unit can't move. Everything you control readies at the start of your turn."],
    ["Remettre droite une carte épuisée : elle peut de nouveau servir (bouger, payer). Au début de ton tour, tout ce que tu contrôles se redresse.", "Turn an exhausted card upright again: it can be used again (move, pay). At the start of your turn, everything you control readies."],
    ["Ta zone à toi : tes unités y arrivent quand tu les joues. On ne s'y bat pas, et elle ne rapporte pas de point.", "Your own zone: your units arrive here when you play them. There is no fighting here, and it scores no points."],
    ["Champ de bataille", "Battlefield"],
    ["Les deux lieux au centre de la table. Chaque champ a son propre effet. Celui qui en garde le contrôle marque des points.", "The two places in the middle of the table. Each battlefield has its own effect. Whoever keeps control of it scores points."],
    ["Conquérir", "Conquer"],
    ["Prendre le contrôle d'un champ de bataille : +1 point, une fois par champ et par tour.", "Take control of a battlefield: +1 point, once per battlefield per turn."],
    ["Contrôler encore un champ au début de ton tour : +1 point pour chaque champ tenu.", "Still control a battlefield at the start of your turn: +1 point for each battlefield held."],
    ["Des unités des deux camps sur le même champ. Chaque camp inflige sa Might totale en dégâts aux unités adverses. Si les deux camps survivent, l'attaquant rentre dans sa base. Les dégâts s'effacent après le combat.", "Units from both sides on the same battlefield. Each side deals its total Might as damage to the enemy units. If both sides survive, the attacker goes back to its base. Damage is removed after the combat."],
    ["Une pause avant qu'un champ change de mains, ou avant les dégâts d'un combat : chacun à son tour, les joueurs peuvent jouer des cartes Action ou Reaction, ou passer.", "A pause before a battlefield changes hands, or before combat damage: taking turns, players can play Action or Reaction cards, or pass."],
    ["Chaîne", "Chain"],
    ["Les cartes et effets qui attendent de se résoudre (colonne « Chaîne »). Le dernier ajouté se résout en premier : c'est ce qui permet de répondre à une carte avant qu'elle fasse effet.", "Cards and effects waiting to resolve (the \"Chain\" column). The last one added resolves first: that's what lets you respond to a card before it takes effect."],
    ["Légende", "Legend"],
    ["La carte qui mène ton deck. Elle reste en jeu toute la partie, a son propre pouvoir et fixe les couleurs (domaines) de ton deck.", "The card that leads your deck. It stays in play all game, has its own power and sets your deck's colours (domains)."],
    ["Une unité spéciale de ta légende, posée à côté d'elle dès le début. Tu peux la jouer comme une carte de ta main.", "A special unit of your legend, set next to it from the start. You can play it like a card from your hand."],
    ["Unité", "Unit"],
    ["Un combattant. Il arrive dans ta base, épuisé, puis peut se déplacer vers les champs de bataille pour les prendre ou les défendre.", "A fighter. It arrives in your base, exhausted, then can move to the battlefields to take or defend them."],
    ["Une carte à usage unique : elle fait son effet puis va dans ta défausse.", "A one-use card: it takes effect, then goes to your trash."],
    ["Équipement", "Gear"],
    ["Une carte qui reste en jeu dans ta base ; certains s'attachent à une unité et lui donnent un bonus.", "A card that stays in play in your base; some attach to a unit and give it a bonus."],
    ["Défausse", "Trash"],
    ["La pile où vont les sorts joués et les unités mortes. Touche une défausse pour voir son contenu.", "The pile where played spells and dead units go. Tap a trash pile to see its contents."],
    ["Ta pioche. Tu pioches 1 carte à chacun de tes tours.", "Your draw pile. You draw 1 card on each of your turns."],
    ["Au début de la partie, tu peux renvoyer jusqu'à 2 cartes de ta main pour en piocher d'autres.", "At the start of the game, you can send back up to 2 cards from your hand to draw others."],
    ["Remettre une carte sous son paquet : sous ton deck pour une carte, sous ta réserve pour une rune.", "Put a card at the bottom of its pile: under your deck for a card, under your rune deck for a rune."],
    ["Retirer une carte de la partie : elle ne revient pas.", "Remove a card from the game: it doesn't come back."],
    ["Points d'expérience gagnés par certaines cartes (Hunt). Ils débloquent les effets Level.", "Experience points gained by some cards (Hunt). They unlock Level effects."],
    ["Une unité ou un objet créé par un effet ; il ne fait pas partie du deck.", "A unit or object created by an effect; it isn't part of the deck."],
    ["Se joue à tout moment, même pendant le tour adverse et en réponse à une autre carte, avant qu'elle se résolve.", "Play it any time, even on the opponent's turn and in response to another card, before it resolves."],
    ["Se joue pendant ton tour, ou pendant n'importe quel showdown (combat compris), même sur le tour adverse.", "Play it on your turn, or during any showdown (combat included), even on the opponent's turn."],
    ["En payant 1 énergie et 1 puissance de plus, l'unité arrive redressée : elle peut bouger tout de suite.", "By paying 1 energy and 1 power more, the unit enters ready: it can move right away."],
    ["L'unité peut se jouer en Reaction, directement sur un champ où tu as déjà des unités.", "The unit can be played as a Reaction, straight to a battlefield where you already have units."],
    ["+X Might tant que l'unité attaque.", "+X Might while the unit is attacking."],
    ["+X Might tant que l'unité défend.", "+X Might while the unit is defending."],
    ["L'unité doit recevoir les dégâts de combat en premier.", "The unit must be assigned combat damage first."],
    ["L'unité reçoit les dégâts de combat en dernier.", "The unit is assigned combat damage last."],
    ["L'adversaire doit payer 1 rune de plus, de n'importe quel type, pour la choisir avec un sort ou une capacité.", "The opponent must pay 1 extra rune of any type to choose it with a spell or ability."],
    ["L'unité peut se déplacer d'un champ de bataille à l'autre, sans repasser par sa base.", "The unit can move from one battlefield to the other without going back to its base."],
    ["Pendant ton tour, pour 1 rune de n'importe quel type, tu peux poser la carte face cachée sur un champ que tu contrôles ; dès le tour suivant, tu peux la jouer en Reaction sans payer son coût.", "On your turn, for 1 rune of any type, you can place the card facedown at a battlefield you control; from the next turn, you can play it as a Reaction without paying its cost."],
    ["Effet en plus si tu as déjà joué une autre carte ce tour.", "Extra effect if you have already played another card this turn."],
    ["Effet qui se déclenche quand l'unité meurt.", "Effect that triggers when the unit dies."],
    ["Quand tu joues la carte, regarde la carte du dessus de ton deck ; tu peux la recycler.", "When you play the card, look at the top card of your deck; you may recycle it."],
    ["Regarde les X cartes du dessus de ton deck ; recycle celles que tu veux et remets les autres dans l'ordre de ton choix.", "Look at the top X cards of your deck; recycle any of them and put the rest back in any order."],
    ["La carte meurt au début du prochain tour de son propriétaire, avant le décompte des points.", "The card dies at the start of its owner's next turn, before scoring."],
    ["Payer le coût indiqué pour attacher cet équipement à une de tes unités, qui reçoit son bonus.", "Pay the cost shown to attach this gear to one of your units, which gets its bonus."],
    ["L'équipement a Reaction et s'attache à une de tes unités quand tu le joues.", "The gear has Reaction and attaches to one of your units when you play it."],
    ["Quand tu joues l'unité, tu peux lui attacher un de tes équipements pour 1 rune de moins.", "When you play the unit, you may attach one of your gear to it for 1 rune less."],
    ["Tu peux payer ce coût en plus pour appliquer une deuxième fois l'effet du sort.", "You may pay this extra cost to apply the spell's effect a second time."],
    ["Quand l'unité conquiert ou tient un champ, tu gagnes X XP.", "When the unit conquers or holds a battlefield, you gain X XP."],
    ["Effet actif tant que tu as au moins ce nombre d'XP.", "Effect active while you have at least this much XP."],
    ["Ton deck ne peut contenir qu'un seul exemplaire de cette carte.", "Your deck can contain only one copy of this card."],
    ["Payer ce coût rend la carte Empowered (une seule fois).", "Paying this cost makes the card Empowered (once)."],
    ["Texte en plus, actif une fois la carte Empowered.", "Extra text, active once the card is Empowered."],
    ["Le sort peut se rejouer depuis ta défausse pour son coût de Flow ; il est ensuite banni.", "The spell can be played again from your trash for its Flow cost; it is then banished."],
    ["L'unité étourdie n'inflige pas de dégâts de combat ce tour.", "The stunned unit deals no combat damage this turn."],
    ["Le joueur met les X cartes du dessus de son deck dans sa défausse.", "The player puts the top X cards of their deck into their trash."],
    ["Une unité est Mighty tant qu'elle a 5 Might ou plus.", "A unit is Mighty while it has 5 or more Might."],
    ["Un bonus permanent de +1 Might posé sur une unité ; une unité n'en porte qu'un à la fois.", "A permanent +1 Might bonus placed on a unit; a unit carries only one at a time."],
    ["Ajoute de l'énergie ou de la puissance à dépenser tout de suite ; on ne peut pas y répondre.", "Adds energy or power to spend right away; it can't be responded to."],
    ["Bienvenue à la table", "Welcome to the table"],
    ["En bas, c'est ton camp ; en haut, celui de Darius. Au milieu (surligné), les deux champs de bataille : c'est là qu'on se bat pour les points. Je t'accompagne à chaque étape : lis, puis touche « Suivant ». Les mots sous l'explication s'expliquent si tu les touches.", "At the bottom is your side; at the top, Darius's. In the middle (highlighted), the two battlefields: that's where you fight for points. I'll walk you through every step: read, then tap \"Next\". The words under the explanation are explained if you tap them."],
    ["Lire une carte", "Reading a card"],
    ["Voici ta main de départ : 4 cartes. La pastille dorée en haut à gauche, c'est le coût en énergie. Touche « Lire » sous une carte (ou passe la souris dessus) pour l'agrandir et lire son texte ; ses mots-clés sont expliqués dessous.", "Here is your starting hand: 4 cards. The gold badge at the top left is the energy cost. Tap \"Read\" under a card (or hover over it) to enlarge it and read its text; its keywords are explained underneath."],
    ["Garde ta main", "Keep your hand"],
    ["Avant de commencer, tu peux renvoyer jusqu'à 2 cartes pour en piocher d'autres : c'est le mulligan. Ta main est bonne : deux Pit Rookie à 2 énergie, jouables tout de suite. Touche « Garder ma main ».", "Before starting, you can send back up to 2 cards to draw others: that's the mulligan. Your hand is good: two Pit Rookies at 2 energy, playable right away. Tap \"Keep my hand\"."],
    ["Ton camp", "Your side"],
    ["Ta rangée (surlignée) : ton score et tes piles (deck, défausse), tes runes, ta base où arrivent tes unités, et ta légende. Ta main est juste en dessous.", "Your row (highlighted): your score and your piles (deck, trash), your runes, your base where your units arrive, and your legend. Your hand is just below."],
    ["Ta légende et ton champion", "Your legend and your champion"],
    ["Maître Yi, ta légende, reste en jeu toute la partie : son pouvoir donne +2 Might à une de tes unités quand elle défend seule. À côté, ton champion Master Yi, Tempered : une unité que tu pourras jouer d'ici, comme une carte de ta main (il coûte 4).", "Master Yi, your legend, stays in play all game: its power gives +2 Might to one of your units when it defends alone. Next to it, your champion Master Yi, Tempered: a unit you can play from here, like a card from your hand (it costs 4)."],
    ["Les runes, ton énergie", "Runes, your energy"],
    ["Tu viens de poser tes 2 premières runes. Pour payer une carte, des runes s'épuisent (elles se couchent) : chacune donne 1 énergie. Au début de chacun de tes tours, elles se redressent et tu en poses 2 de plus.", "You have just put down your first 2 runes. To pay for a card, runes exhaust (they turn sideways): each gives 1 energy. At the start of each of your turns, they ready and you put down 2 more."],
    ["Joue une unité", "Play a unit"],
    ["Tu as pioché ta carte du tour. Les cartes à cadre vert sont celles que tu peux jouer maintenant. Pit Rookie coûte 2 : c'est exactement ce que tu as. Touche-le puis « Jouer », ou glisse-le dans ta base. (Zhonya's Hourglass est un équipement : garde-le pour plus tard.)", "You drew your card for the turn. Cards with a green frame are the ones you can play now. Pit Rookie costs 2: exactly what you have. Tap it then \"Play\", or drag it into your base. (Zhonya's Hourglass is a gear: keep it for later.)"],
    ["Une unité arrive épuisée", "A unit enters exhausted"],
    ["Ton Pit Rookie est dans ta base, couché : il est épuisé. Il ne peut pas bouger ce tour-ci ; il se redressera au début de ton prochain tour. Son effet (« buff » une autre unité) n'a rien fait : tu n'avais pas d'autre unité.", "Your Pit Rookie is in your base, turned sideways: it is exhausted. It can't move this turn; it will ready at the start of your next turn. Its effect (\"buff\" another unit) did nothing: you had no other unit."],
    ["Termine ton tour", "End your turn"],
    ["Tes 2 runes sont épuisées : plus d'énergie. Touche le gros bouton « Terminer le tour ». Ensuite, c'est à Darius de jouer.", "Your 2 runes are exhausted: no energy left. Tap the big \"End turn\" button. Then it's Darius's turn."],
    ["Un nouveau tour", "A new turn"],
    ["Darius joue son tour. Les flèches corail montrent ses coups, et le bouton « Journal » les raconte. Si le jeu te demande de répondre, touche « Passer ».", "Darius is playing his turn. The coral arrows show his moves, and the \"Log\" button describes them. If the game asks you to respond, tap \"Pass\"."],
    ["C'est de nouveau à toi. Chaque tour commence pareil : tes cartes se redressent, tes champs tenus marquent, tu poses 2 runes, tu pioches 1 carte. Ensuite tu joues autant de cartes que ton énergie le permet, dans l'ordre que tu veux.", "It's your turn again. Every turn starts the same way: your cards ready, battlefields you hold score, you put down 2 runes, you draw 1 card. Then you play as many cards as your energy allows, in any order you like."],
    ["Les champs de bataille", "The battlefields"],
    ["Deux champs : Seat of Power (le tien) et Trifarian War Camp (celui de Darius). Chacun a un effet : touche son image pour le lire. Une unité qui arrive sur un champ libre le conquiert : +1 point.", "Two battlefields: Seat of Power (yours) and Trifarian War Camp (Darius's). Each has an effect: tap its picture to read it. A unit that arrives on a free battlefield conquers it: +1 point."],
    ["Va sur un champ de bataille", "Go to a battlefield"],
    ["Glisse une unité de ta base vers un champ (ou touche-la, puis « Déplacer » et le champ). Se déplacer épuise l'unité. Astuce : First Mate (3 énergie) redresse une autre unité en arrivant, qui peut donc encore bouger.", "Drag a unit from your base to a battlefield (or tap it, then \"Move\" and the battlefield). Moving exhausts the unit. Tip: First Mate (3 energy) readies another unit when it enters, so that unit can still move."],
    ["Marque un point", "Score a point"],
    ["Tenir un champ", "Hold a battlefield"],
    ["Ton point est marqué. Au début de chacun de tes tours, chaque champ que tu contrôles encore rapporte +1 point : tu le tiens. Darius va essayer de te le reprendre ; le premier à 8 points gagne.", "Your point is scored. At the start of each of your turns, each battlefield you still control scores +1 point: you hold it. Darius will try to take it back; the first to 8 points wins."],
    ["Gagne un combat", "Win a combat"],
    ["Un combat a lieu quand des unités des deux camps sont sur le même champ. Va attaquer un champ où Darius a des unités, ou défends le tien quand il vient. La plus grosse Might totale l'emporte ; Punch First (+5 Might) peut renverser un combat.", "A combat happens when units from both sides are on the same battlefield. Attack a battlefield where Darius has units, or defend yours when he comes. The greater total Might wins; Punch First (+5 Might) can turn a combat around."],
    ["Tu connais les bases", "You know the basics"],
    ["Bravo ! Pour la suite : touche une carte pour lire son texte et ses mots-clés, « Conseil » montre ce que l'IA jouerait à ta place, « ↶ Reprendre » annule ton dernier coup. Le lexique explique tous les mots du jeu.", "Well done! From here: tap a card to read its text and keywords, \"Hint\" shows what the AI would play in your place, \"↶ Undo\" takes back your last move. The glossary explains every word of the game."],
    ["Une question", "A question"],
    ["Une carte te demande un choix : réponds dans la bulle au milieu de la table.", "A card asks you to choose: answer in the bubble in the middle of the table."],
    ["Un sort jouable", "A playable spell"],
    ["Un sort de ta main a un cadre vert : tu peux le jouer. Il fait son effet une fois, puis va dans ta défausse. Son coût « 1+2 » : 1 énergie et 2 puissance de sa couleur, payée en recyclant des runes.", "A spell in your hand has a green frame: you can play it. It takes effect once, then goes to your trash. Its \"1+2\" cost: 1 energy and 2 power of its colour, paid by recycling runes."],
    ["Ton champion est jouable", "Your champion is playable"],
    ["Tu as assez d'énergie pour Master Yi, Tempered (4), ton champion : il a un cadre vert à côté de ta légende. Touche-le pour le jouer.", "You have enough energy for Master Yi, Tempered (4), your champion: it has a green frame next to your legend. Tap it to play it."],
    ["Combat !", "Combat!"],
    ["Avant les dégâts, chacun peut encore jouer des cartes Action ou Reaction. Puis chaque camp inflige sa Might totale aux unités adverses ; une unité meurt si elle reçoit au moins sa Might en dégâts. Si les deux camps survivent, l'attaquant rentre dans sa base.", "Before damage, each player can still play Action or Reaction cards. Then each side deals its total Might to the enemy units; a unit dies if it takes at least its Might in damage. If both sides survive, the attacker goes back to its base."],
    ["Darius avance", "Darius advances"],
    ["Darius a une unité sur un champ de bataille. S'il le tient au début de son tour, il marque 1 point. Pour l'en chasser, envoie plus de Might que lui.", "Darius has a unit on a battlefield. If he holds it at the start of his turn, he scores 1 point. To drive him out, send more Might than he has."],
    ["Darius marque", "Darius scores"],
    ["Chaque champ pris ou tenu rapporte des points, à lui aussi. Reprends-lui ses champs !", "Every battlefield taken or held scores points, for him too. Take his battlefields back!"],
    ["Ta défausse", "Your trash"],
    ["Une de tes cartes est dans ta défausse (un sort joué ou une unité morte). Touche la pile de défausse (surlignée) pour la revoir.", "One of your cards is in your trash (a played spell or a dead unit). Tap the trash pile (highlighted) to see it again."],
    ["Bienvenue dans Riftbound", "Welcome to Riftbound"],
    ["Riftbound est un jeu de cartes pour deux joueurs. Chacun a un deck de cartes et une légende, le héros qui mène son armée. Ce tutoriel part de zéro : aucune connaissance n'est nécessaire, tout s'explique pas à pas.", "Riftbound is a card game for two players. Each has a deck of cards and a legend, the hero who leads their army. This tutorial starts from zero: no knowledge is needed, everything is explained step by step."],
    ["Le but : 8 points", "The goal: 8 points"],
    ["Au centre de la table, deux champs de bataille. Tes unités vont les prendre : prendre un champ rapporte 1 point, le garder jusqu'à ton tour suivant aussi. Le premier à 8 points gagne.", "In the middle of the table, two battlefields. Your units will take them: taking a battlefield scores 1 point, and keeping it until your next turn does too. The first to 8 points wins."],
    ["Trois sortes de cartes", "Three kinds of cards"],
    ["Une unité est un combattant qui reste sur la table. Un sort fait son effet une fois, puis va dans la défausse. Un équipement reste en jeu et peut renforcer une unité. Chaque carte a un coût, et chaque unité une force : sa Might.", "A unit is a fighter that stays on the table. A spell takes effect once, then goes to the trash. A gear stays in play and can strengthen a unit. Every card has a cost, and every unit a strength: its Might."],
    ["Les runes paient tes cartes", "Runes pay for your cards"],
    ["Ton énergie vient de tes runes : à chaque tour, tu en poses 2 nouvelles. Pour jouer une carte, tu épuises (couches) autant de runes que son coût. Au début de ton tour suivant, elles se redressent.", "Your energy comes from your runes: each turn, you put down 2 new ones. To play a card, you exhaust (turn sideways) as many runes as its cost. At the start of your next turn, they ready."],
    ["Un tour de jeu", "A turn"],
    ["Chaque tour se déroule toujours dans le même ordre :", "Every turn always goes in the same order:"],
    ["Tes cartes épuisées se redressent.", "Your exhausted cards ready."],
    ["Chaque champ que tu tiens rapporte 1 point.", "Each battlefield you hold scores 1 point."],
    ["Tu poses 2 runes.", "You put down 2 runes."],
    ["Tu pioches 1 carte.", "You draw 1 card."],
    ["Tu joues : poser des unités, lancer des sorts, déplacer tes unités vers les champs.", "You play: put down units, cast spells, move your units to the battlefields."],
    ["Tu termines ton tour : à l'adversaire.", "You end your turn: over to the opponent."],
    ["Les combats", "Combats"],
    ["Quand tes unités arrivent sur un champ où sont celles de l'adversaire, il y a combat. Chaque camp inflige autant de dégâts que la Might totale de ses unités ; une unité meurt si elle reçoit au moins sa Might en dégâts. Le camp qui reste prend ou garde le champ.", "When your units arrive on a battlefield where the opponent's units are, there is a combat. Each side deals as much damage as the total Might of its units; a unit dies if it takes at least its Might in damage. The side that remains takes or keeps the battlefield."],
    ["À toi de jouer", "Your turn to play"],
    ["Tu joues Maître Yi contre Darius, une IA débutante, dans une vraie partie. Le coach te guide étape par étape, avec six défis à réussir :", "You play Master Yi against Darius, a beginner AI, in a real game. The coach guides you step by step, with six challenges to complete:"],
    ["Puissance", "Power"],
    ["Redresser", "Ready"],
    ["Tenir", "Hold"],
    ["Recycler", "Recycle"],
    ["Bannir", "Banish"],
    ["Jeton", "Token"],
    ["Sort", "Spell"],
    ["Lexique", "Glossary"],
    ["Lexique : tous les mots du jeu", "Glossary: every word of the game"],
    ["Touche un mot :", "Tap a word:"],
    ["Les bases", "The basics"],
    ["Mots-clés des cartes", "Card keywords"],
    ["Les mots du jeu, puis les mots-clés écrits [ainsi] sur les cartes. Sur la table, touche une carte (ou passe la souris dessus) : ses mots-clés sont expliqués sous son texte.", "The words of the game, then the keywords written [like this] on cards. On the table, tap a card (or hover over it): its keywords are explained under its text."],
    ["Coach · étape ${}/${}", "Coach · step ${}/${}"],
    ["Patiente un instant", "Hold on a moment"],
    ["Continue la partie : la suite arrive au bon moment.", "Keep playing: the next step comes at the right moment."],
    ["Ensuite : ${}.", "Next: ${}."],
    ["Terminer le tutoriel", "Finish the tutorial"],
    ["Écran ${} sur ${}", "Screen ${} of ${}"],
    ["Bloqué ? « Conseil » montre un bon coup, « ↶ Reprendre » annule le dernier.", "Stuck? \"Hint\" shows a good move, \"↶ Undo\" takes back the last one."],
    ["Tu sais jouer une unité, prendre et tenir un champ, et gagner un combat. Le reste (sorts, réactions, mots-clés) s'apprend en jouant : touche une carte pour lire ses mots-clés, et le bouton « Conseil » est là quand tu hésites.", "You know how to play a unit, take and hold a battlefield, and win a combat. The rest (spells, reactions, keywords) is learnt by playing: tap a card to read its keywords, and the \"Hint\" button is there when you hesitate."],
    // ---------------------------------------------------------------- menu, tableau de bord
    ["deck introuvable", "deck not found"], ["Ton deck d'entraînement", "Your training deck"], ["Aucun deck choisi", "No deck chosen"],
    ["Choisis la légende et la liste que tu veux travailler. Le tableau de bord te la proposera à chaque partie et suivra ton bilan avec.",
     "Choose the legend and the list you want to practise. The dashboard will offer it for every game and track your record with it."],
    ["Choisir mon deck", "Choose my deck"], ["Créer mon deck", "Create my deck"], ["Chargement…", "Loading…"],
    ["Ce deck n'est pas jouable en l'état (cartes non modélisées ou deck incomplet).", "This deck isn't playable as it is (cards not modelled or incomplete deck)."],
    ["Modifier", "Edit"], ["⚔ Jouer avec ce deck", "⚔ Play with this deck"], ["Changer", "Change"], ["Partie en cours", "Game in progress"],
    ["Tour ${} · ${}", "Turn ${} · ${}"], ["La partie t'attend sur la table.", "The game is waiting for you on the table."], ["▶ Reprendre", "▶ Resume"],
    ["Dernière partie", "Last game"], ["Tour ${}. Elle est encore sur la table.", "Turn ${}. It is still on the table."], ["Revoir la table", "See the table"],
    ["${} contre ${} · tour ${} · ${}", "${} vs ${} · turn ${} · ${}"], ["Toutes mes parties", "All my games"],
    ["Nouveau sur Riftbound ? Le tutoriel t'apprend les bases en une dizaine de minutes.", "New to Riftbound? The tutorial teaches you the basics in about ten minutes."],
    ["Chargement de tes parties…", "Loading your games…"], ["Pas encore de partie", "No game yet"], ["★ Apprendre à jouer", "★ Learn to play"],
    ["Bilan avec ce deck : ${} victoire${}, ${} défaite${} (${} partie${} finie${})", "Record with this deck: ${} win${}, ${} loss${} (${} game${} finished)"],
    ["Pas encore de partie finie avec ce deck.", "No finished game with this deck yet."], ["Chargement des légendes…", "Loading the legends…"],
    ["Touche une légende pour voir ses decks.", "Tap a legend to see its decks."], ["mon deck · ${}", "my deck · ${}"], ["pas encore jouable", "not playable yet"],
    ["Le deck que tu veux travailler. Tu pourras le changer quand tu veux.", "The deck you want to practise. You can change it whenever you like."],
    ["Tu préfères ta propre liste ? Construis-la dans l'éditeur, elle apparaîtra ici sous sa légende.", "Prefer your own list? Build it in the editor, it will appear here under its legend."],
    ["Créer un deck", "Create a deck"], ["Contre l'IA : choisis les deux decks et le niveau.", "Against the AI: choose both decks and the level."],
    ["Apprendre à jouer", "Learn to play"], ["Tutoriel : une partie guidée en 6 défis.", "Tutorial: a guided game in 6 challenges."],
    ["Cartes, runes, battlefields, import et export.", "Cards, runes, battlefields, import and export."], ["Modifier ou jouer directement.", "Edit or play directly."],
    ["Mes parties", "My games"], ["Revois-les coup par coup.", "Review them move by move."], ["Aide", "Help"], ["Règles de la table.", "Table rules."],
    ["Choisis ton deck, joue contre l'IA, relis tes parties.", "Choose your deck, play against the AI, review your games."],
    ["Aide : règles de la table", "Help: table rules"], ["Débutant ?", "Beginner?"],
    ["« Apprendre à jouer » (menu) : une partie guidée en 6 défis.", "\"Learn to play\" (menu): a guided game in 6 challenges."],
    ["But.", "Goal."], ["Marquer les points de victoire avant l'IA en contrôlant les battlefields. Le moteur applique les Core Rules complètes ; une même donne donne toujours la même partie.",
     "Score the victory points before the AI by controlling the battlefields. The engine applies the complete Core Rules; the same deal always gives the same game."],
    ["Jouer.", "Play."], ["Touche une carte à cadre vert pour voir ses actions, ou glisse-la vers sa zone ou sa cible. Les questions (cibles, effets optionnels, mulligan) s'affichent sur le plateau.",
     "Tap a card with a green frame to see its actions, or drag it to its zone or target. Questions (targets, optional effects, mulligan) show on the board."],
    ["Fin de tour.", "End of turn."],
    ["Le gros bouton vert termine ton tour ; en réaction ou en showdown il devient « Passer ». Raccourci : barre d'espace, qui répond aussi « Oui » aux questions oui / non (« Utiliser cet effet ? »).",
     "The big green button ends your turn; in a reaction or a showdown it becomes \"Pass\". Shortcut: space bar, which also answers \"Yes\" to yes / no questions (\"Use this effect?\")."],
    ["Reprendre.", "Undo."],
    ["« ↶ Reprendre » (touche Retour arrière) annule ton dernier coup, autant de fois que tu veux. En duel, seulement tant que tu n'as pas passé le focus : ni après « Passer » ou « Terminer le tour », ni une fois que ton ami a joué.",
     "\"↶ Undo\" (Backspace key) takes back your last move, as many times as you like. In a duel, only as long as you haven't passed the focus: not after \"Pass\" or \"End turn\", nor once your friend has played."],
    ["Conseil.", "Hint."], ["Le bouton « Conseil » simule deux tours : l'écart est en points, 7 vaut environ un point, sous 2 c'est du bruit.",
     "The \"Hint\" button simulates two turns: the gap is in points, 7 is worth about one point, under 2 it's noise."],
    ["L'IA.", "The AI."], ["Elle ne voit pas ta main. Niveau « Normal » : celle des simulations ; « Fort » : 3 simulations par coup, plus lente. Flèches corail : ses derniers coups.",
     "It doesn't see your hand. \"Normal\" level: the one used in simulations; \"Strong\": 3 simulations per move, slower. Coral arrows: its last moves."],
    ["Journal et défausse.", "Log and trash."], ["Le bouton Journal ouvre l'historique ; touche une défausse pour voir son contenu.", "The Log button opens the history; tap a trash pile to see its contents."],
    ["Decks.", "Decks."], ["Seules les cartes modélisées par le moteur sont jouables ; les autres sont grisées (« à modéliser »).",
     "Only cards modelled by the engine are playable; the others are greyed out (\"to be modelled\")."],
    ["Jouer avec un ami.", "Play with a friend."],
    ["L'un crée une room (BO1 ou BO3) et donne le code ou le lien ; l'autre le tape dans « Rejoindre ». Pas de conseil en duel ; « Reprendre » tant que tu gardes la main. Si l'invité recharge sa page, la partie reprend où elle en était ; si l'hôte ferme la sienne, la room est perdue.",
     "One player creates a room (BO1 or BO3) and gives the code or the link; the other types it in \"Join\". No hints in a duel; \"Undo\" as long as you keep the turn. If the guest reloads their page, the game resumes where it was; if the host closes theirs, the room is lost."],
    ["Donne.", "Deal."], ["Même numéro de donne = même partie. Laisse vide pour une donne au hasard.", "Same deal number = same game. Leave empty for a random deal."],
    ["sans légende", "no legend"], ["${} · ${}/40 cartes${} · ${}", "${} · ${}/40 cards${} · ${}"],
    ["Aucun deck pour l'instant. Les decks de référence (Akali G2, LeBlanc IQ#5…) sont proposés dans « Nouvelle partie ».",
     "No deck yet. The reference decks (Akali G2, LeBlanc IQ#5…) are offered in \"New game\"."],
    ["Les decks sont gardés sur cet appareil, et dans la base claude.ai quand elle existe.", "Decks are kept on this device, and in the claude.ai database when it exists."],
    ["Lecture des parties impossible pour l'instant.", "Can't read the games right now."], ["en cours", "in progress"],
    ["Télécharger cette partie", "Download this game"], ["${} · donne ${} · tour ${} · ${}", "${} · deal ${} · turn ${} · ${}"],
    ["Aucune partie enregistrée pour l'instant. Elles apparaissent ici dès que tu joues.", "No saved game yet. They show up here as soon as you play."],
    ["Télécharge-les (toutes, ou une seule avec ⤓) puis dépose le fichier dans le dépôt (riftbound/train/games/games/) : le workflow « Apprentissage » les rejoue et les analyse.",
     "Download them (all of them, or a single one with ⤓) then drop the file in the repository (riftbound/train/games/games/): the \"Apprentissage\" workflow replays and analyses them."],
    ["Tout télécharger", "Download all"], ["Toi", "You"], ["IA ${}", "${} AI"],
    // ---------------------------------------------------------------- relevés du robot verif_anglais.mjs
    ["Sons : ${}", "Sounds: ${}"], ["Choisir", "Choose"], ["2 manches gagnantes", "best of 3"], ["normale", "normal"], ["forte", "strong"],
    ["${} commence", "${} goes first"], ["ex. ${}", "e.g. ${}"], ["Jouables seulement", "Playable only"], ["bannie", "banned"],
    ["Retirer", "Remove"], ["Autres", "Other"], ["Champions", "Champions"],
    ["Unités (${})", "Units (${})"], ["Sorts (${})", "Spells (${})"], ["Équipements (${})", "Gear (${})"], ["Autres (${})", "Other (${})"],
    ["${} carte${}. Touche une carte pour l'ajouter au", "${} card${}. Tap a card to add it to the"],
    ["(légendes, runes et battlefields vont toujours au deck) ; les cartes grisées ne sont pas encore modélisées.",
     "(legends, runes and battlefields always go to the deck); greyed-out cards aren't modelled yet."],
    ["main vide", "empty hand"], ["libre", "free"], ["Toi (${})", "You (${})"], ["${} (épuisée)", "${} (exhausted)"],
    ["Déclenchement · ${}", "Trigger · ${}"], ["Sur la chaîne · ${}", "On the chain · ${}"], ["Suivant", "Next"], ["Compris", "Got it"],
    ["Revoir coup par coup", "Review move by move"], ["Fin", "End"], ["duel, room ${}", "duel, room ${}"],
    ["Garder ${}", "Keep ${}"], ["Ne pas faire entrer ${}", "Don't bring in ${}"], ["Sortir ${}", "Take out ${}"],
    ["Faire entrer ${}", "Bring in ${}"], ["Sideboard vide.", "Empty sideboard."], ["${} contre ${} · ${}", "${} vs ${} · ${}"],
    ["▶ Lecture", "▶ Play"], ["⏸ Pause", "⏸ Pause"], ["${} défi${} réussi${} sur ${}", "${1} challenge${2} complete out of ${4}"],
    ["${} · donne ${} · BO3 manche ${} · tour ${} · ${}", "${} · deal ${} · BO3 game ${} · turn ${} · ${}"],
    ["Replay · donne ${} · ${} contre ${} · BO3 manche ${} · ${}", "Replay · deal ${} · ${} vs ${} · BO3 game ${} · ${}"],
    ["adverse", "enemy"], ["adverse · ${}", "enemy · ${}"], ["à toi · ${}", "yours · ${}"],
  ];

  // Motifs écrits à la main : textes assemblés en plusieurs morceaux dans src.html.
  const ORD = n => n + (n % 100 >= 11 && n % 100 <= 13 ? "th" : ["th", "st", "nd", "rd"][n % 10] || "th");
  const P = [
    [/^(.*) · coût (.*)$/, "$1 · cost $2"],   // en-tête de l'agrandi d'une carte
    [/^(.*) Touche une carte ou une zone en surbrillance\.$/, (m, t) => t(m[1]) + " Tap a highlighted card or zone."],
    [/^(.*)\((\d+)e\)(.*)$/, (m, t) => t(m[1]) + "(" + ORD(+m[2]) + ")" + t(m[3])],
    [/^Défausse de l'IA \((.+)\) \((\d+)\)$/, "AI's trash ($1) ($2)"],
    [/^Défausse de (.+) \((\d+)\)$/, "$1's trash ($2)"],
    [/^Défausse à toi \((\d+)\)$/, "Your trash ($1)"],
    [/^Cartes bannies de l'IA \((.+)\) \((\d+)\)$/, "AI's banished cards ($1) ($2)"],
    [/^Cartes bannies à toi \((\d+)\)$/, "Your banished cards ($1)"],
    [/^(.+) \(bannie\)$/, (m, t) => t(m[1]) + " (banished)"],
    [/^(.+) : pas encore de deck jouable$/, "$1: no playable deck yet"],
    [/^(.+) \(pas encore de deck\)$/, "$1 (no deck yet)"],
    [/^Main de (.+) \((\d+)\)$/, "$1's hand ($2)"],
    [/^Ta main \((\d+)\)$/, "Your hand ($1)"],
    [/^Jouer → (.+)$/, (m, t) => "Play → " + t(m[1])],
    [/^(\d+) partie(s?)( gardées?)?( dans ce navigateur)?$/, (m) => m[1] + " game" + m[2] + (m[3] ? " kept" : "") + (m[4] ? " in this browser" : "")],
    [/^(★ liste conseillée · )?liste de tournoi(, sideboard (\d+))?( · pas encore jouable)?$/,
      (m) => (m[1] ? "★ recommended list · " : "") + "tournament list" + (m[2] ? ", sideboard " + m[3] : "") + (m[4] ? " · not playable yet" : "")],
    [/^sideboard (\d+)$/, "sideboard $1"],
    [/^(hôte|invité) : (.*)$/, (m, t) => (m[1] === "hôte" ? "host" : "guest") + ": " + t(m[2])],
    [/^Bilan avec ce deck : (\d+) victoires?, (\d+) défaites? \((\d+) parties? finies?\)$/,
      (m) => `Record with this deck: ${m[1]} win${m[1] === "1" ? "" : "s"}, ${m[2]} loss${m[2] === "1" ? "" : "es"} (${m[3]} game${m[3] === "1" ? "" : "s"} finished)`],
    [/^(\d+) défis? réussis? sur (\d+)(\.?)$/, (m) => `${m[1]} challenge${m[1] === "1" ? "" : "s"} complete out of ${m[2]}${m[3]}`],
    [/^(\d+) échanges?$/, (m) => m[1] + " swap" + (m[1] === "1" ? "" : "s")],
    [/^Ici, (.*)$/, (m, t) => "Here, " + t(m[1])],
    [/^(Victoire|Défaite) (\d+-\d+)$/, (m) => (m[1] === "Victoire" ? "Victory " : "Defeat ") + m[2]],
    [/^(Victoire|Défaite|Partie nulle) (\d+)-(\d+)$/, (m) => ({ Victoire: "Victory", "Défaite": "Defeat", "Partie nulle": "Draw" })[m[1]] + " " + m[2] + "-" + m[3]],
  ];
  // diagnostics du duel (journal de connexion) : morceaux après « hôte : » / « invité : »
  D.push(["room ouverte (${})", "room open (${})"], ["connexion entrante (encodage ${})", "incoming connection (encoding ${})"],
    ["canal ouvert avec l'invité", "channel open with the guest"], ["erreur du canal : ${}", "channel error: ${}"], ["canal fermé", "channel closed"],
    ["erreur PeerJS ${}", "PeerJS error ${}"], ["refus : ${}", "refusal: ${}"], ["match envoyé à ${}", "match sent to ${}"],
    ["connecté au serveur, appel de la room ${}", "connected to the server, calling room ${}"], ["canal ouvert, envoi du hello", "channel open, sending hello"],
    ["l'hôte ne répond pas", "the host isn't answering"], ["pas de réponse de l'hôte, nouvel envoi du hello (${})", "no answer from the host, sending hello again (${})"],
    ["envoi impossible : ${}", "can't send: ${}"], ["reçu « ${} »", "received \"${}\""], ["erreur : ${}", "error: ${}"]);

  // ---------------------------------------------------------------- moteur de traduction
  const EX = new Map(), PATS = [];
  const norm = s => s.replace(/\s+/g, " ").trim();
  const reEsc = s => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  for (const [fr, en] of D) {
    const k = norm(fr);
    if (!k.includes("${}")) { EX.set(k, en); continue; }
    const parts = k.split("${}");
    const re = new RegExp("^" + parts.map(reEsc).join("([\\s\\S]*?)") + "$");
    let n = 0;
    const out = en.replace(/\$\{(\d?)\}/g, (_, d) => "\u0001" + (d ? d : String(++n)) + "\u0001");
    PATS.push([re, (m, t) => out.replace(/\u0001(\d)\u0001/g, (_, i) => t(m[+i] || "")), parts.join("").length]);
  }
  PATS.sort((a, b) => b[2] - a[2]);
  const ALL = P.map(([re, out]) => [re, typeof out === "function" ? out : (m, t) => out.replace(/\$(\d)/g, (_, i) => t(m[+i] || ""))]).concat(PATS);
  const CACHE = new Map();
  function core(s) {
    if (CACHE.has(s)) return CACHE.get(s);
    let r = EX.has(s) ? EX.get(s) : null;
    if (r === null) for (const [re, f] of ALL) { const m = s.match(re); if (m) { r = f(m, sub); break; } }
    if (CACHE.size > 20000) CACHE.clear();
    CACHE.set(s, r);
    return r;
  }
  // tx : traduction d'un texte (espaces de bord gardés), ou null s'il n'y en a pas
  function tx(s) {
    if (!s || !/[A-Za-zÀ-ÿ]/.test(s)) return null;
    const m = /^(\s*)([\s\S]*?)(\s*)$/.exec(s), c = norm(m[2]);
    const r = core(c);
    return r === null ? null : m[1] + r + m[3];
  }
  function sub(s) { const r = tx(s); return r === null ? s : r; }

  const I18N = window.I18N = {
    lang, tx, t: sub,
    set(l) {
      if (l !== "en" && l !== "fr") return;
      try { localStorage.setItem("rbt-lang", l); } catch (e) {}
      const u = new URL(location.href); u.searchParams.delete("lang");
      location.replace(u.href.replace(/[?]$/, "") + (u.search ? "&" : "?") + "lang=" + l);
    },
  };
  document.documentElement.lang = lang;
  // bouton de langue (menu) : <button data-lang="en">
  document.addEventListener("click", e => { const b = e.target.closest && e.target.closest("[data-lang]"); if (b) I18N.set(b.dataset.lang); });
  if (lang !== "en") return;

  // ---------------------------------------------------------------- application à la page
  const ATTRS = ["title", "aria-label", "placeholder", "label"];
  const SKIP = new Set(["SCRIPT", "STYLE", "TEXTAREA"]);
  function attrs(el) {
    for (const a of ATTRS) {
      const v = el.getAttribute(a);
      if (v) { const r = tx(v); if (r !== null && r !== v) el.setAttribute(a, r); }
    }
  }
  function walk(n) {
    if (n.nodeType === 3) {
      const p = n.parentNode;
      if (p && SKIP.has(p.nodeName)) return;
      const r = tx(n.data);
      if (r !== null && r !== n.data) n.data = r;
    } else if (n.nodeType === 1 || n.nodeType === 11) {
      if (n.nodeType === 1) { if (SKIP.has(n.nodeName)) return; attrs(n); }
      for (let c = n.firstChild; c; c = c.nextSibling) walk(c);
    }
  }
  I18N.walk = walk;
  // traduction immédiate après innerHTML / textContent / insertAdjacentHTML (avant les mesures de mise en page)
  const hook = (proto, prop) => {
    const d = Object.getOwnPropertyDescriptor(proto, prop);
    if (!d || !d.set) return;
    Object.defineProperty(proto, prop, Object.assign({}, d, { set(v) { d.set.call(this, v); walk(this); } }));
  };
  hook(Element.prototype, "innerHTML");
  hook(Node.prototype, "textContent");
  const iah = Element.prototype.insertAdjacentHTML;
  Element.prototype.insertAdjacentHTML = function (pos, html) { iah.call(this, pos, html); walk(pos === "beforebegin" || pos === "afterend" ? this.parentNode || this : this); };
  const conf = window.confirm;
  window.confirm = msg => conf.call(window, sub(String(msg)));
  new MutationObserver(recs => {
    for (const r of recs) {
      if (r.type === "childList") r.addedNodes.forEach(walk);
      else if (r.type === "characterData") walk(r.target);
      else if (r.type === "attributes" && r.target.nodeType === 1) attrs(r.target);
    }
  }).observe(document.documentElement, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ATTRS });
  // textes posés par CSS (content)
  const st = document.createElement("style");
  st.textContent = `.mull button[aria-pressed="true"]::after { content: "back"; } .vtile.p1::before { content: "YOU"; } .vtile.cpu::after { content: "AI"; }`;
  document.head.appendChild(st);
  if (document.body) walk(document.body);
  else document.addEventListener("DOMContentLoaded", () => walk(document.body));
})();

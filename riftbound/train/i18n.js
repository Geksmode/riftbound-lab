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
    ["Ta main", "Your hand"], ["(épuisée)", "(exhausted)"],
    ["Deck : ${} cartes. Dessus connu (Predict, Vision) : ${}. Touche pour voir.", "Deck: ${} cards. Known top (Predict, Vision): ${}. Tap to see."],
    ["Deck : ${} cartes", "Deck: ${} cards"], ["Défausse : touche pour voir les ${} cartes", "Trash: tap to see the ${} cards"],
    ["Prêtes", "Ready"], ["Réserve ${}", "Reserve ${}"], ["Runes", "Runes"], ["Base", "Base"], ["Légende", "Legend"],
    ["cachée", "hidden"], ["${} cachées", "${} hidden"], ["COPIE", "COPY"], ["JETON", "TOKEN"],
    ["· coût ${}", "· cost ${}"], ["· might ${}", "· might ${}"], ["· bonus +${}", "· bonus +${}"],
    ["Ce coup n'est pas possible.", "This move isn't possible."], ["Cette unité n'est pas une cible possible.", "This unit isn't a possible target."],
    ["à toi", "yours"], ["Jouer", "Play"], ["Jouer → ${}", "Play → ${}"],
    ["Déplacer… (puis choisir la destination)", "Move… (then choose the destination)"], ["Lire la carte", "Read the card"],
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
    // ---------------------------------------------------------------- tutoriel
    ["Garde ta main", "Keep your hand"],
    ["Tu as pioché 4 cartes. Avant de commencer, tu peux en renvoyer jusqu'à 2 pour en repiocher (le mulligan). Ta main est bonne : deux Pit Rookie qui coûtent 2. Touche « Garder ma main ».",
     "You drew 4 cards. Before starting, you can send back up to 2 to draw new ones (the mulligan). Your hand is good: two Pit Rookies that cost 2. Tap \"Keep my hand\"."],
    ["Joue une unité", "Play a unit"],
    ["À côté de ta légende : tes 2 runes. Chaque rune s'épuise pour donner 1 énergie. Pit Rookie coûte 2 (chiffre en haut à gauche de la carte). Touche-le (cadre vert) puis « Jouer », ou glisse-le dans ta base.",
     "Next to your legend: your 2 runes. Each rune exhausts to give 1 energy. Pit Rookie costs 2 (number at the top left of the card). Tap it (green frame) then \"Play\", or drag it into your base."],
    ["Termine ton tour", "End your turn"],
    ["Une unité arrive épuisée (couchée) : elle pourra bouger au prochain tour. Tu n'as plus d'énergie : touche le gros bouton « Terminer le tour ». Darius joue ensuite.",
     "A unit enters exhausted (turned sideways): it can move next turn. You have no energy left: tap the big \"End turn\" button. Darius plays next."],
    ["Va sur un champ de bataille", "Go to a battlefield"],
    ["Nouveau tour : tes unités se redressent, tu poses 2 runes de plus et tu pioches. Glisse une unité de ta base vers un des deux champs de bataille au centre (ou touche-la puis « Déplacer »). Bouger épuise l'unité. Astuce : First Mate (3) redresse une autre unité en arrivant, qui peut donc encore bouger.",
     "New turn: your units ready, you put down 2 more runes and you draw. Drag a unit from your base to one of the two battlefields in the middle (or tap it then \"Move\"). Moving exhausts the unit. Tip: First Mate (3) readies another unit when it enters, so that unit can still move."],
    ["Marque un point", "Score a point"],
    ["Prendre le contrôle d'un champ, c'est le conquérir : +1 point. Au début de chacun de tes tours, chaque champ que tu tiens encore rapporte +1 point. Le premier à 8 points gagne.",
     "Taking control of a battlefield is conquering it: +1 point. At the start of each of your turns, each battlefield you still hold scores +1 point. The first to 8 points wins."],
    ["Gagne un combat", "Win a combat"],
    ["Va sur un champ où Darius a des unités, ou défends le tien. Chaque camp inflige des dégâts égaux à sa Might (sa force) ; une unité meurt quand elle reçoit autant de dégâts que sa Might. Le camp qui reste garde le champ. Punch First donne +5 Might pendant un combat, et ta légende Maître Yi donne +2 Might à une unité qui défend seule.",
     "Go to a battlefield where Darius has units, or defend yours. Each side deals damage equal to its Might (its strength); a unit dies when it takes as much damage as its Might. The side that remains keeps the battlefield. Punch First gives +5 Might during a combat, and your legend Master Yi gives +2 Might to a unit that defends alone."],
    ["Fenêtre de réaction", "Reaction window"],
    ["Une carte va se résoudre ou un combat s'ouvre : tu peux répondre avec une carte Reaction (ou Action pendant un combat). Sinon, touche « Passer ».",
     "A card is about to resolve or a combat is starting: you can respond with a Reaction card (or Action during a combat). Otherwise, tap \"Pass\"."],
    ["Une question", "A question"], ["Une carte te demande un choix : réponds dans la bulle au milieu du plateau.", "A card asks you to choose: answer in the bubble in the middle of the board."],
    ["Darius a une unité sur un champ de bataille. S'il le tient au début de son tour, il marque 1 point. Pour l'en chasser, envoie plus de Might que lui.",
     "Darius has a unit on a battlefield. If he holds it at the start of his turn, he scores 1 point. To drive him out, send more Might than he has."],
    ["Chaque champ pris ou tenu rapporte des points, à lui aussi. Reprends-lui ses champs !", "Every battlefield taken or held scores points, for him too. Take his battlefields back!"],
    ["Huit points pour gagner", "Eight points to win"],
    ["Deux joueurs, deux champs de bataille au centre. Prendre un champ, puis le tenir, rapporte des points. Le premier à 8 points gagne.",
     "Two players, two battlefields in the middle. Taking a battlefield, then holding it, scores points. The first to 8 points wins."],
    ["Les runes, ton énergie", "Runes, your energy"],
    ["Chaque tour, tu poses 2 runes et tu pioches 1 carte. Une rune s'épuise pour 1 énergie : c'est avec elle que tu paies tes cartes.",
     "Each turn, you put down 2 runes and draw 1 card. A rune exhausts for 1 energy: that's what you pay for your cards with."],
    ["Unités et combats", "Units and combats"],
    ["Joue des unités dans ta base, puis envoie-les sur les champs. Deux camps sur le même champ : combat, et la plus grosse force (Might) l'emporte.",
     "Play units in your base, then send them to the battlefields. Two sides on the same battlefield: combat, and the greater strength (Might) wins."],
    ["À toi de jouer", "Your turn to play"], ["Tu joues", "You play"], ["Maître Yi", "Master Yi"], ["contre", "against"],
    [", une IA débutante. Six défis, une dizaine de minutes :", ", a beginner AI. Six challenges, about ten minutes:"],
    ["Bloqué ? « Conseil » montre le meilleur coup, « ↶ Reprendre » annule le dernier.", "Stuck? \"Hint\" shows the best move, \"↶ Undo\" takes back the last one."],
    ["Retour", "Back"], ["Passer l'intro", "Skip the intro"], ["Jouer la partie guidée", "Play the guided game"],
    ["Tutoriel · ${} contre ${} (IA débutante)", "Tutorial · ${} vs ${} (beginner AI)"], ["Défi réussi : ${} !", "Challenge complete: ${}!"],
    ["Tutoriel réussi", "Tutorial complete"], ["Tutoriel réussi !", "Tutorial complete!"],
    ["Tu connais les bases. Finis la partie jusqu'à 8 points, ou lance une vraie partie depuis le menu.", "You know the basics. Finish the game up to 8 points, or start a real game from the menu."],
    ["Défi en cours : ${}.", "Current challenge: ${}."], ["Défi ${} · ${}", "Challenge ${} · ${}"],
    ["Victoire ${}-${} !", "Victory ${}-${}!"], ["Défaite ${}-${}", "Defeat ${}-${}"], ["Partie terminée", "Game over"],
    ["Tu sais jouer une unité, prendre et tenir un champ, et gagner un combat. Le reste (sorts, réactions, mots-clés) s'apprend en jouant : le bouton « Conseil » est là pour ça.",
     "You know how to play a unit, take and hold a battlefield, and win a combat. The rest (spells, reactions, keywords) is learnt by playing: the \"Hint\" button is there for that."],
    ["Tu peux recommencer le tutoriel : même donne, même main de départ.", "You can restart the tutorial: same deal, same starting hand."],
    ["Continuer la partie", "Continue the game"], ["${} défi${} réussi${} sur ${}.", "${1} challenge${2} complete out of ${4}."],
    ["Recommencer le tuto", "Restart the tutorial"], ["Partie contre l'IA", "Game against the AI"],
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
    [/^(.*) Touche une carte ou une zone en surbrillance\.$/, (m, t) => t(m[1]) + " Tap a highlighted card or zone."],
    [/^(.*)\((\d+)e\)(.*)$/, (m, t) => t(m[1]) + "(" + ORD(+m[2]) + ")" + t(m[3])],
    [/^Défausse de l'IA \((.+)\) \((\d+)\)$/, "AI's trash ($1) ($2)"],
    [/^Défausse de (.+) \((\d+)\)$/, "$1's trash ($2)"],
    [/^Défausse à toi \((\d+)\)$/, "Your trash ($1)"],
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

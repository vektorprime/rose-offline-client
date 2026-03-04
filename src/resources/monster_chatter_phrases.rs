use bevy::prelude::{Resource, Reflect};

use crate::components::ClientEntityType;

/// Resource defining chat phrases for monsters and NPCs
#[derive(Resource, Reflect)]
pub struct MonsterChatterPhrases {
    /// Phrases specifically for hostile monsters
    pub monster_phrases: MonsterPhrases,
    /// Phrases specifically for friendly NPCs
    pub npc_phrases: NpcPhrases,
}

/// Phrases for hostile monsters
#[derive(Clone, Reflect)]
pub struct MonsterPhrases {
    /// Combat/aggressive phrases
    pub combat_phrases: Vec<String>,
    /// Defensive/scared phrases (when hurt)
    pub defensive_phrases: Vec<String>,
    /// Bored/idle phrases
    pub bored_phrases: Vec<String>,
    /// Hungry phrases
    pub hungry_phrases: Vec<String>,
    /// Confident/taunting phrases
    pub confident_phrases: Vec<String>,
    /// Monster-specific Rose Online themed phrases
    pub rose_monster_phrases: Vec<String>,
}

/// Phrases for friendly NPCs
#[derive(Clone, Reflect)]
pub struct NpcPhrases {
    /// Greeting phrases
    pub greeting_phrases: Vec<String>,
    /// Idle/chat phrases
    pub idle_phrases: Vec<String>,
    /// Helpful phrases
    pub helpful_phrases: Vec<String>,
    /// NPC-specific Rose Online themed phrases
    pub rose_npc_phrases: Vec<String>,
}

impl Default for MonsterChatterPhrases {
    fn default() -> Self {
        Self::new()
    }
}

impl MonsterChatterPhrases {
    /// Get a random phrase based on entity type
    pub fn get_random_phrase(&self, entity_type: ClientEntityType) -> &String {
        match entity_type {
            ClientEntityType::Monster => self.get_random_monster_phrase(),
            ClientEntityType::Npc => self.get_random_npc_phrase(),
            _ => &self.monster_phrases.combat_phrases[0], // Fallback
        }
    }

    /// Get a random monster phrase from all monster categories
    fn get_random_monster_phrase(&self) -> &String {
        let all: Vec<&String> = self.monster_phrases.get_all_phrases();
        if all.is_empty() {
            &self.monster_phrases.combat_phrases[0]
        } else {
            let index = (rand::random::<f32>() * all.len() as f32) as usize;
            all[index]
        }
    }

    /// Get a random NPC phrase from all NPC categories
    fn get_random_npc_phrase(&self) -> &String {
        let all: Vec<&String> = self.npc_phrases.get_all_phrases();
        if all.is_empty() {
            &self.npc_phrases.greeting_phrases[0]
        } else {
            let index = (rand::random::<f32>() * all.len() as f32) as usize;
            all[index]
        }
    }

    pub fn new() -> Self {
        Self {
            monster_phrases: MonsterPhrases {
                combat_phrases: vec![
                    "I'll get you!".to_string(),
                    "You can't hide from me!".to_string(),
                    "Fresh meat!".to_string(),
                    "Prepare to die!".to_string(),
                    "You'll make a fine snack!".to_string(),
                    "No escape!".to_string(),
                    "I've been waiting for you...".to_string(),
                    "Your equipment will be mine!".to_string(),
                    "Weakling!".to_string(),
                    "You dare challenge me?".to_string(),
                    "Your Zulie will be mine!".to_string(),
                    "Feel the wrath of the wild!".to_string(),
                    "You adventurers are all the same!".to_string(),
                    "I'll crush you like a Jelly Bean!".to_string(),
                    "No mercy for surface dwellers!".to_string(),
                    "The shadows favor me!".to_string(),
                    "You've trespassed too far!".to_string(),
                    "Your cart won't save you now!".to_string(),
                    "I've slain stronger than you!".to_string(),
                    "Your bones will decorate my lair!".to_string(),
                    "More experience for me!".to_string(),
                    "I haven't eaten in ages...".to_string(),
                    "Finally, a challenger!".to_string(),
                ],
                defensive_phrases: vec![
                    "Please don't hurt me!".to_string(),
                    "I'm just minding my own business...".to_string(),
                    "Why me?".to_string(),
                    "Help! Someone help!".to_string(),
                    "I surrender!".to_string(),
                    "Take my items, just let me live!".to_string(),
                    "I have a family!".to_string(),
                    "Not the face!".to_string(),
                    "I just wanted to see Luna...".to_string(),
                    "I'll give you all my Zulie!".to_string(),
                    "Please, I'm just a humble creature!".to_string(),
                    "The guards will hear about this!".to_string(),
                    "I'm too young to respawn!".to_string(),
                    "Can't we just talk this out?".to_string(),
                ],
                bored_phrases: vec![
                    "So bored...".to_string(),
                    "Anyone there?".to_string(),
                    "I should have stayed in bed...".to_string(),
                    "Is it lunch time yet?".to_string(),
                    "My feet hurt...".to_string(),
                    "When does my shift end?".to_string(),
                    "I miss the sunshine...".to_string(),
                    "La la la...".to_string(),
                    "Same spawns, different day...".to_string(),
                    "I need a hobby...".to_string(),
                    "Do these patrols ever end?".to_string(),
                    "Counting rocks... 1, 2, 3...".to_string(),
                    "Maybe I'll take up fishing...".to_string(),
                    "I've been standing here for hours...".to_string(),
                ],
                hungry_phrases: vec![
                    "I'm so hungry...".to_string(),
                    "That player looks tasty...".to_string(),
                    "When's dinner?".to_string(),
                    "Got any snacks?".to_string(),
                    "I could go for some Luna fruit...".to_string(),
                    "Those HP potions look delicious...".to_string(),
                    "Is that a sandwich in your inventory?".to_string(),
                    "I haven't eaten since last respawn...".to_string(),
                ],
                confident_phrases: vec![
                    "Is that all you've got?".to_string(),
                    "Pathetic!".to_string(),
                    "Come closer...".to_string(),
                    "You call that a weapon?".to_string(),
                    "I've seen scarier squirrels!".to_string(),
                    "I've defeated champions!".to_string(),
                    "Your guild can't help you here!".to_string(),
                    "I eat level 100s for breakfast!".to_string(),
                    "That armor won't save you!".to_string(),
                    "You'll need more than buffs!".to_string(),
                ],
                rose_monster_phrases: vec![
                    // Planet lore - aggressive monster perspective
                    "The Akram Kingdom will fall!".to_string(),
                    "Junon's forests belong to us!".to_string(),
                    "Luna's ice makes my claws sharper...".to_string(),
                    "Eldeon's jungles hide our kind...".to_string(),
                    // Job system - taunting players about their class
                    "Just a weak Visitor, how pathetic...".to_string(),
                    "You'll never survive past Visitor status!".to_string(),
                    // Soldier branch taunts
                    "Soldiers are just meat shields!".to_string(),
                    "Knights hide behind their armor like cowards...".to_string(),
                    "Champions are all rage, no strategy!".to_string(),
                    // Muse branch taunts
                    "Muses and their annoying music...".to_string(),
                    "Clerics can't heal stupid!".to_string(),
                    "Mages are squishy without mana!".to_string(),
                    // Hawker branch taunts
                    "Hawkers run away like cowards...".to_string(),
                    "Scouts are useless up close!".to_string(),
                    "Raiders hide in shadows because they're weak!".to_string(),
                    // Dealer branch taunts
                    "Dealers are only good for their money...".to_string(),
                    "Bourgeois buy their way to victory!".to_string(),
                    "Artisans can't fight to save their lives!".to_string(),
                    // Location taunts
                    "Valley of Luxem Tower will be your grave!".to_string(),
                    "El Verloon Desert is scorching today...".to_string(),
                    "Anima Lake has the strangest creatures...".to_string(),
                    "Gorge of Silence is eerily quiet...".to_string(),
                    "Desert of the Dead is a PvP warzone...".to_string(),
                    "Oblivion Temple holds dark secrets...".to_string(),
                    // Dungeon threats
                    "Cave of Ulverick is not for beginners...".to_string(),
                    "Halls of Oblivion echo with whispers...".to_string(),
                    // Combat
                    "Training Grounds is where I learned to fight...".to_string(),
                    "Junon Cartel battles are intense...".to_string(),
                    "PvP maps are not for the faint of heart...".to_string(),
                    // Monster life
                    "We respawn, you know...".to_string(),
                    "Being a monster isn't so bad...".to_string(),
                    "At least I don't have to grind XP!".to_string(),
                    "We have feelings too, you know!".to_string(),
                    // Lore
                    "The Mana Stream flows through everything...".to_string(),
                    "This world has so many secrets...".to_string(),
                    "The Seven Planets hold many mysteries...".to_string(),
                ],
            },
            npc_phrases: NpcPhrases {
                greeting_phrases: vec![
                    "Welcome, adventurer!".to_string(),
                    "Greetings, traveler!".to_string(),
                    "Hello there!".to_string(),
                    "Good day to you!".to_string(),
                    "May Arua bless your journey!".to_string(),
                    "Welcome to our humble village!".to_string(),
                    "Well met, friend!".to_string(),
                    "How can I help you today?".to_string(),
                    "Nice to see a friendly face!".to_string(),
                    "Travel safely!".to_string(),
                ],
                idle_phrases: vec![
                    "...".to_string(),
                    "Hmmm...".to_string(),
                    "*yawns*".to_string(),
                    "Another day in the Seven Hearts...".to_string(),
                    "*stretches*".to_string(),
                    "Business has been slow lately...".to_string(),
                    "I wonder what's happening in Junon Polis...".to_string(),
                    "The weather is lovely today!".to_string(),
                    "I heard there's a party in Zant...".to_string(),
                    "These old bones ache...".to_string(),
                    "La la la...".to_string(),
                    "I need a vacation...".to_string(),
                    "Did you hear that?".to_string(),
                    "Nice weather we're having!".to_string(),
                ],
                helpful_phrases: vec![
                    "Need any supplies?".to_string(),
                    "I have the finest wares in town!".to_string(),
                    "Looking for a quest?".to_string(),
                    "Be careful out there!".to_string(),
                    "The monsters have been restless lately...".to_string(),
                    "Have you visited the Item Merchant?".to_string(),
                    "The Weapon Dealer has new stock!".to_string(),
                    "Ferrell Guild can help with transportation!".to_string(),
                    "Need to refine your equipment?".to_string(),
                    "I can help you with that!".to_string(),
                    "Talk to me if you need anything!".to_string(),
                    "Safe travels, adventurer!".to_string(),
                ],
                rose_npc_phrases: vec![
                    // Planet lore - friendly NPC perspective
                    "The Seven Hearts shall rise again!".to_string(),
                    "Have you visited Junon Polis lately?".to_string(),
                    "Adventurer's Plains is perfect for new Visitors...".to_string(),
                    "I dream of visiting Eldeon someday...".to_string(),
                    "The glow of Luna is beautiful at night...".to_string(),
                    "Junon's permanent spring is so lovely...".to_string(),
                    // Luna lore
                    "The Goddess Lunar has never known love...".to_string(),
                    "Magic was created from the essence of the universe...".to_string(),
                    "Luna's ice and snow are beautiful but deadly...".to_string(),
                    "Crystal Snowfields sparkle like diamonds!".to_string(),
                    "The Forgotten Temple holds ancient secrets...".to_string(),
                    "Mana Snowfields are treacherous this time of year...".to_string(),
                    "Mt Eruca towers over the Magic City...".to_string(),
                    "Sea of Dawn dungeon is not for the weak...".to_string(),
                    "Cost me 5000 zulie to take the Flying Vessel here...".to_string(),
                    // Eldeon lore
                    "Xita Refuge is the only safe place on Eldeon...".to_string(),
                    "Shady Jungle hides many dangers...".to_string(),
                    "Forest of Wandering lives up to its name...".to_string(),
                    "Sikuku Ruins are dangerous...".to_string(),
                    "Marsh of Ghosts gives me the creeps...".to_string(),
                    "Sikuku Catacombs hold the toughest bosses...".to_string(),
                    // Other planets
                    "I heard Skaaj is lovely this time of year...".to_string(),
                    "Orlo's mysteries call to me...".to_string(),
                    "Karkia remains unexplored by most...".to_string(),
                    "Hebarn is said to be the final planet...".to_string(),
                    // Job encouragement
                    "Every Visitor must choose their path...".to_string(),
                    "Soldiers make great protectors...".to_string(),
                    "Knights have the best defense...".to_string(),
                    "Champions are powerful warriors!".to_string(),
                    "Muses have beautiful music...".to_string(),
                    "Clerics keep parties alive...".to_string(),
                    "Mages deal devastating magic damage!".to_string(),
                    "Hawkers are swift and agile...".to_string(),
                    "Scouts excel at ranged combat...".to_string(),
                    "Raiders strike from the shadows!".to_string(),
                    "Dealers know the value of Zulie...".to_string(),
                    "Bourgeois are wealthy merchants...".to_string(),
                    "Artisans craft the finest equipment!".to_string(),
                    // Junon locations
                    "Valley of Luxem Tower is full of adventure...".to_string(),
                    "Canyon City of Zant has the best markets...".to_string(),
                    "Breezy Hills are perfect for picnics...".to_string(),
                    "Kenji Beach is perfect for swimming!".to_string(),
                    "Dolphin Island broke away ages ago...".to_string(),
                    "Forest of Wisdom has ancient knowledge...".to_string(),
                    // Luna locations
                    "Magic City of the Eucar is breathtaking...".to_string(),
                    "Arumic Valley has rare herbs...".to_string(),
                    "Freezing Plateau will freeze your bones!".to_string(),
                    // Dungeons
                    "Cave of Ulverick is not for beginners...".to_string(),
                    "Halls of Oblivion echo with whispers...".to_string(),
                    // Transportation
                    "I love riding my cart around...".to_string(),
                    "Castle Gear is so expensive though...".to_string(),
                    "The Flying Vessel travels between planets...".to_string(),
                    "Cart Schematics are hard to find...".to_string(),
                    "The Mana Engine powers the Flying Vessel...".to_string(),
                    // Faction references
                    "Seven Hearts forever!".to_string(),
                    "The era of Hearts brought great change...".to_string(),
                    // Activities
                    "Party hunting is the best way to level...".to_string(),
                    "The drop rate today is terrible...".to_string(),
                    "I got a unique drop yesterday!".to_string(),
                    "Refining equipment takes skill...".to_string(),
                    // Quests
                    "Rosemary's Doll quest took forever...".to_string(),
                    "Those collection quests take ages...".to_string(),
                    "Repeatable quests are good for farming...".to_string(),
                    // Economy
                    "Zulie doesn't grow on trees...".to_string(),
                    "5000 zulie just to travel to Luna!".to_string(),
                    "The Ferrell Guild overcharges...".to_string(),
                    "Item Merchants never have good stock...".to_string(),
                    // Gems and crafting
                    "Gems make equipment so much stronger...".to_string(),
                    "Need more materials for crafting...".to_string(),
                    "Rare metals are hard to come by...".to_string(),
                    // Lore
                    "The goddess Arua watches over us...".to_string(),
                    "Flight Generator changed everything in year 658...".to_string(),
                    "This world has so many secrets...".to_string(),
                    "Steam Shock era brought great advances...".to_string(),
                    // Events
                    "Christmas Event brings such joy!".to_string(),
                    "Winter Event has the best rewards...".to_string(),
                    "Event mounts are so rare...".to_string(),
                ],
            },
        }
    }
}

impl MonsterPhrases {
    /// Get all monster phrases combined
    pub fn get_all_phrases(&self) -> Vec<&String> {
        let mut all = Vec::new();
        all.extend(self.combat_phrases.iter());
        all.extend(self.defensive_phrases.iter());
        all.extend(self.bored_phrases.iter());
        all.extend(self.hungry_phrases.iter());
        all.extend(self.confident_phrases.iter());
        all.extend(self.rose_monster_phrases.iter());
        all
    }
}

impl NpcPhrases {
    /// Get all NPC phrases combined
    pub fn get_all_phrases(&self) -> Vec<&String> {
        let mut all = Vec::new();
        all.extend(self.greeting_phrases.iter());
        all.extend(self.idle_phrases.iter());
        all.extend(self.helpful_phrases.iter());
        all.extend(self.rose_npc_phrases.iter());
        all
    }
}

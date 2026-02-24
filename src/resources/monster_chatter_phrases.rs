use bevy::prelude::{Resource, Reflect};

/// Resource defining chat phrases for monsters
#[derive(Resource, Reflect)]
pub struct MonsterChatterPhrases {
    /// Default phrases for all monsters
    pub default_phrases: Vec<String>,
    /// Combat/aggressive phrases
    pub combat_phrases: Vec<String>,
    /// Defensive/scared phrases
    pub defensive_phrases: Vec<String>,
    /// Bored/idle phrases
    pub bored_phrases: Vec<String>,
    /// Hungry phrases
    pub hungry_phrases: Vec<String>,
    /// Confident/taunting phrases
    pub confident_phrases: Vec<String>,
    /// Funny/random phrases
    pub funny_phrases: Vec<String>,
    /// Rose Online themed phrases
    pub rose_themed_phrases: Vec<String>,
}

impl Default for MonsterChatterPhrases {
    fn default() -> Self {
        Self::new()
    }
}

impl MonsterChatterPhrases {
    /// Get all phrases combined
    pub fn get_all_phrases(&self) -> Vec<&String> {
        let mut all = Vec::new();
        all.extend(self.default_phrases.iter());
        all.extend(self.combat_phrases.iter());
        all.extend(self.defensive_phrases.iter());
        all.extend(self.bored_phrases.iter());
        all.extend(self.hungry_phrases.iter());
        all.extend(self.confident_phrases.iter());
        all.extend(self.funny_phrases.iter());
        all.extend(self.rose_themed_phrases.iter());
        all
    }

    /// Get a random phrase from all categories
    pub fn get_random_phrase(&self) -> &String {
        let all = self.get_all_phrases();
        if all.is_empty() {
            &self.default_phrases[0]
        } else {
            let index = (rand::random::<f32>() * all.len() as f32) as usize;
            all.get(index).unwrap_or(&all[0])
        }
    }

    pub fn new() -> Self {
        Self {
            default_phrases: vec![
                "...".to_string(),
                "Hmmm...".to_string(),
            ],
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
            ],
            hungry_phrases: vec![
                "I'm so hungry...".to_string(),
                "That player looks tasty...".to_string(),
                "When's dinner?".to_string(),
                "Got any snacks?".to_string(),
            ],
            confident_phrases: vec![
                "Is that all you've got?".to_string(),
                "Pathetic!".to_string(),
                "Come closer...".to_string(),
                "You call that a weapon?".to_string(),
                "I've seen scarier squirrels!".to_string(),
            ],
            funny_phrases: vec![
                "Did you hear that?".to_string(),
                "I think I left the oven on...".to_string(),
                "Nice weather we're having".to_string(),
                "I need a vacation...".to_string(),
                "This isn't my real job, you know".to_string(),
                "My mom says I'm special".to_string(),
                "I'm not paid enough for this".to_string(),
                "Have you seen my pet?".to_string(),
                "I dropped my wallet somewhere...".to_string(),
                "These rocks are uncomfortable".to_string(),
            ],
            rose_themed_phrases: vec![
                "The Seven Hearts shall rise again!".to_string(),
                "Have you visited Junon lately?".to_string(),
                "I heard there's treasure in the caves...".to_string(),
                "Watch out for the Dragon King!".to_string(),
                "The Akram Kingdom will fall!".to_string(),
                "I dream of visiting Eldeon someday...".to_string(),
                "The glow of Luna is beautiful at night...".to_string(),
                "Have you seen the fairy queen?".to_string(),
            ],
        }
    }
}

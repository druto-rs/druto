//! Grammar rules: irregular noun mappings and English suffix inflection tables.

/// Known irregular English noun pairs (singular, plural).
pub(crate) static IRREGULAR_NOUNS: &[(&str, &str)] = &[
    ("alumna", "alumnae"),
    ("alumnus", "alumni"),
    ("analysis", "analyses"),
    ("automaton", "automata"),
    ("axis", "axes"),
    ("bacillus", "bacilli"),
    ("bacterium", "bacteria"),
    ("basis", "bases"),
    ("cactus", "cacti"),
    ("child", "children"),
    ("crisis", "crises"),
    ("criterion", "criteria"),
    ("curriculum", "curricula"),
    ("datum", "data"),
    ("diagnosis", "diagnoses"),
    ("die", "dice"),
    ("focus", "foci"),
    ("foot", "feet"),
    ("fungus", "fungi"),
    ("goose", "geese"),
    ("index", "indices"),
    ("louse", "lice"),
    ("man", "men"),
    ("matrix", "matrices"),
    ("medium", "media"),
    ("memorandum", "memoranda"),
    ("mouse", "mice"),
    ("move", "moves"),
    ("nucleus", "nuclei"),
    ("ox", "oxen"),
    ("parenthesis", "parentheses"),
    ("person", "people"),
    ("phenomenon", "phenomena"),
    ("prognosis", "prognoses"),
    ("quiz", "quizzes"),
    ("sex", "sexes"),
    ("stadium", "stadia"),
    ("stimulus", "stimuli"),
    ("stratum", "strata"),
    ("syllabus", "syllabi"),
    ("symposium", "symposia"),
    ("synopsis", "synopses"),
    ("thesis", "theses"),
    ("tooth", "teeth"),
    ("vertex", "vertices"),
    ("vortex", "vortices"),
    ("woman", "women"),
    ("zombie", "zombies"),
];

/// Looks up an irregular plural form for a lowercase singular word.
pub(crate) fn find_irregular_plural(word_lower: &str) -> Option<&'static str> {
    for &(singular, plural) in IRREGULAR_NOUNS {
        if singular == word_lower {
            return Some(plural);
        }
    }
    None
}

/// Looks up an irregular singular form for a lowercase plural word.
pub(crate) fn find_irregular_singular(word_lower: &str) -> Option<&'static str> {
    for &(singular, plural) in IRREGULAR_NOUNS {
        if plural == word_lower {
            return Some(singular);
        }
    }
    None
}

/// Inflects a singular lowercase word into its plural form using English grammar suffix rules.
pub(crate) fn pluralize_suffix(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }

    // -fe endings: knife -> knives, life -> lives, wife -> wives
    if let Some(prefix) = word.strip_suffix("fe")
        && (prefix.ends_with("kni") || prefix.ends_with("li") || prefix.ends_with("wi"))
    {
        let mut result = String::with_capacity(prefix.len() + 3);
        result.push_str(prefix);
        result.push_str("ves");
        return result;
    }

    // -f endings: leaf -> leaves, half -> halves, wolf -> wolves, loaf -> loaves, shelf -> shelves, thief -> thieves, calf -> calves
    if let Some(prefix) = word.strip_suffix('f')
        && (prefix.ends_with("lea")
            || prefix.ends_with("hal")
            || prefix.ends_with("wol")
            || prefix.ends_with("loa")
            || prefix.ends_with("shel")
            || prefix.ends_with("thie")
            || prefix.ends_with("cal")
            || prefix.ends_with("el")
            || prefix.ends_with("scar"))
    {
        let mut result = String::with_capacity(prefix.len() + 3);
        result.push_str(prefix);
        result.push_str("ves");
        return result;
    }

    // -y endings: category -> categories, day -> days
    if let Some(prefix) = word.strip_suffix('y') {
        match prefix.chars().last() {
            Some(preceding_char) if is_vowel(preceding_char) => {
                let mut result = String::with_capacity(word.len() + 1);
                result.push_str(word);
                result.push('s');
                return result;
            }
            Some(_) => {
                let mut result = String::with_capacity(prefix.len() + 3);
                result.push_str(prefix);
                result.push_str("ies");
                return result;
            }
            None => {}
        }
    }

    // -s, -x, -z, -ch, -sh endings: bus -> buses, box -> boxes, dish -> dishes, church -> churches
    if word.ends_with("ss")
        || word.ends_with('x')
        || word.ends_with('z')
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        let mut result = String::with_capacity(word.len() + 2);
        result.push_str(word);
        result.push_str("es");
        return result;
    }

    // -us endings: status -> statuses, virus -> viruses, bus -> buses
    if word.ends_with("us") {
        let mut result = String::with_capacity(word.len() + 2);
        result.push_str(word);
        result.push_str("es");
        return result;
    }

    // -o endings preceded by consonant: hero -> heroes, tomato -> tomatoes, echo -> echoes
    if let Some(prefix) = word.strip_suffix('o')
        && (word == "hero"
            || word == "potato"
            || word == "tomato"
            || word == "echo"
            || word == "veto"
            || word == "torpedo"
            || word == "embargo")
    {
        let mut result = String::with_capacity(prefix.len() + 3);
        result.push_str(prefix);
        result.push_str("oes");
        return result;
    }

    // Default regular rule: append 's'
    let mut result = String::with_capacity(word.len() + 1);
    result.push_str(word);
    result.push('s');
    result
}

/// Inflects a plural lowercase word into its singular form using English grammar suffix rules.
pub(crate) fn singularize_suffix(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }

    // -ies endings: categories -> category, days -> day
    if let Some(prefix) = word.strip_suffix("ies")
        && let Some(preceding_char) = prefix.chars().last()
        && !is_vowel(preceding_char)
    {
        let mut result = String::with_capacity(prefix.len() + 1);
        result.push_str(prefix);
        result.push('y');
        return result;
    }

    // -ves endings: knives -> knife, halves -> half
    if let Some(prefix) = word.strip_suffix("ves") {
        if prefix.ends_with("kni") || prefix.ends_with("li") || prefix.ends_with("wi") {
            let mut result = String::with_capacity(prefix.len() + 2);
            result.push_str(prefix);
            result.push_str("fe");
            return result;
        }
        if prefix.ends_with("lea")
            || prefix.ends_with("hal")
            || prefix.ends_with("wol")
            || prefix.ends_with("loa")
            || prefix.ends_with("shel")
            || prefix.ends_with("thie")
            || prefix.ends_with("cal")
            || prefix.ends_with("el")
            || prefix.ends_with("scar")
        {
            let mut result = String::with_capacity(prefix.len() + 1);
            result.push_str(prefix);
            result.push('f');
            return result;
        }
    }

    // -es endings on sibilants: boxes -> box, churches -> church, dishes -> dish, glasses -> glass
    if let Some(prefix) = word.strip_suffix("es") {
        if prefix.ends_with('x')
            || prefix.ends_with("ch")
            || prefix.ends_with("sh")
            || prefix.ends_with("ss")
            || prefix.ends_with("zz")
        {
            return prefix.to_string();
        }

        // -oes endings: heroes -> hero, tomatoes -> tomato
        if prefix == "hero"
            || prefix == "potato"
            || prefix == "tomato"
            || prefix == "echo"
            || prefix == "veto"
            || prefix == "torpedo"
            || prefix == "embargo"
        {
            return prefix.to_string();
        }

        // -uses endings: statuses -> status, viruses -> virus, buses -> bus
        if prefix == "status" || prefix == "virus" || prefix == "bus" {
            return prefix.to_string();
        }
    }

    // Singular words ending in -us, -is, -ss, or specific exceptions
    if word.ends_with("us")
        || word.ends_with("is")
        || word.ends_with("ss")
        || word == "gas"
        || word == "canvas"
    {
        return word.to_string();
    }

    // Default regular rule: strip trailing 's'
    if let Some(prefix) = word.strip_suffix('s') {
        return prefix.to_string();
    }

    word.to_string()
}

#[inline]
fn is_vowel(character: char) -> bool {
    matches!(
        character,
        'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suffix_rules_empty() {
        assert_eq!(pluralize_suffix(""), "");
        assert_eq!(singularize_suffix(""), "");
    }
}

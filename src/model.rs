use std::collections::HashMap;
use std::ops::{AddAssign, Mul};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Serialize, Deserialize};


#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub source: String,
}

impl Message {
    pub fn build (&self, chat_mode: bool) -> String {
        if chat_mode {
            let opener = if self.source == "Fiona" { "<|assistant|>" } else { "<|user|>" };
            format!("{opener} {} </s>", self.content)
        } else {
            format!("{}: {}", self.source, self.content)
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Memory {
    pub id: usize,
    pub content: String,
    pub source: String,
    pub links: HashMap<usize, f64>,
}


#[derive(Serialize, Deserialize)]
pub struct MemoryConstruct (HashMap<usize, Memory>, usize);

impl MemoryConstruct {
    pub fn new () -> MemoryConstruct {
        MemoryConstruct (HashMap::<usize, Memory>::new(), 0)
    }

    pub fn put (&mut self, content: String, source: String, id: usize) {
        self.0.insert(
            id,
            Memory {
                id,
                content,
                source,
                links: HashMap::new(),
            }
        );
    }

    pub fn push_message (&mut self, message: Message, id: usize) {
        if id > self.1 {
            self.1 = id;
        }
        self.0.insert(
            id,
            Memory {
                id,
                content: message.content,
                source: message.source,
                links: HashMap::new(),
            }
        );
    }

    pub fn link (&mut self, from: usize, to: usize, weight: f64) {
        println!("Attempting to link {from} to {to} with weight {weight}");
        self.0
            .get_mut(&from)
            .unwrap()
            .links
            .entry(to)
            .or_insert(0.)
            .add_assign(weight);
    }

    pub fn search (&self, query: String) -> Vec<Memory> {
        if self.0.is_empty () { return vec!() }

        println!("querying for {query}");

        let matcher: SkimMatcherV2 = SkimMatcherV2::default();
        let hit = self.0.iter()
            .max_by_key(|(_, v)| matcher.fuzzy_match(&query, &v.content))
            .expect("should have at least one since I already checked if it's empty");

        let mut hits = vec!(hit.0.clone());

        let max_hit = hit.1.links.iter().max_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((_, &w)) = max_hit {
            hits.extend(hit.1.links.iter().filter(|l| l.1 > &w.mul(0.9)).map(|l| l.0.clone()));
        }

        hits.iter().map(|h| self.0.get(h).unwrap().clone()).collect::<Vec<Memory>>()
    }

    pub fn get_last_id (&self) -> usize {
        self.1
    }
}

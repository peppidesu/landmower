use std::{collections::{hash_map, HashMap}, hash::{Hash as _, Hasher as _}, path::Path};

use serde::{Deserialize, Serialize};
use base64::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub link: String,
    pub metadata: EntryMetadata
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EntryMetadata {
    pub used: u64,
    pub last_used: std::time::SystemTime,
    pub created: std::time::SystemTime
}

impl From<String> for Entry {
    fn from(link: String) -> Self {
        Self {
            link,
            metadata: EntryMetadata {
                used: 0,
                last_used: std::time::SystemTime::now(),
                created: std::time::SystemTime::now()
            }
        }
    }
}



/// Stores alias->link mappings and the reverse mapping.
#[derive(Clone, Debug, Default)]
pub struct Links { 
    /// Forward hashmap is used for finding the associated link for a given alias.
    forward_map: HashMap<String, Entry>, 
    /// Inverse of the forward hashmap.
    /// The forward mapping is surjective, so each link can have multiple associated aliases.
    /// 
    /// Note: might be worth benching to see if linear search is actually slower.
    reverse_map: HashMap<String, Vec<String>>,
}

impl Links {
    /// Load link data from the given file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {        
        let path = path.as_ref();

        if !path.exists() {
            // Create the directory if it doesn't exist.
            std::fs::create_dir_all(
                path.parent()
                    .ok_or(format!("Invalid link data path: '{}'", path.display()))?
            ).map_err(|e| format!("Could not create directory: {e}"))?;
            
            // Create empty link storage & write to file
            let result: Self = Self { 
                forward_map: HashMap::new(), 
                reverse_map: HashMap::new() 
            };
            result.save(path)?;
            Ok(result)
        } else {
            // Read file contents
            let data = std::fs::read_to_string(path)
                .map_err(|e| format!("Could not load links: {e}"))?;

            let forward_map: HashMap<String, Entry> = toml::from_str(&data).unwrap();

            // Build reverse lookup
            let mut reverse_map: HashMap<String, Vec<String>> = HashMap::new();
            for (k, v) in &forward_map {
                if reverse_map.contains_key(&v.link) {
                    // link already has associated key; add to existing list
                    reverse_map.get_mut(&v.link).unwrap().push(k.clone());
                } else {
                    // create a new entry for this link
                    reverse_map.insert(v.link.clone(), vec![k.clone()]);
                }
            }
            Ok(Self { forward_map, reverse_map })
        }
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.forward_map.get(key)
    }
    
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Entry> {
        self.forward_map.get_mut(key)
    }

    /// Insert a new mapping with a generated key and the given link.
    ///
    /// ## Errors
    ///
    /// This function will return an error if the key is already in use, a.k.a. the link
    /// already has an associated mapping 
    pub fn add(&mut self, link: String) -> (String, Entry) {
        match self.generate_key(&link) {
            Ok(key) => (key.clone(), self.add_named(key, link).unwrap()),
            Err(pair) => pair
        }
    }
    
    fn generate_key(&self, link: &str) -> Result<String, (String, Entry)> {
        // hash + base64 encode
        let mut hasher = std::hash::DefaultHasher::new();
        link.hash(&mut hasher);
        let hash = BASE64_URL_SAFE_NO_PAD.encode(hasher.finish().to_le_bytes());

        // take first 4 characters, keep adding if there is a collision
        for i in 4..=hash.len() {
            let key = &hash[..i];
            if let Some(other) = self.forward_map.get(key) { 
                if other.link == link {
                    return Err((key.to_string(), other.clone()));
                }
                continue;
            }
            return Ok(key.into());
        }
        let other = self.get(&hash).unwrap().clone();
        Err((hash, other)) // hash collision -> link already present in storage
    }

    /// Insert a new mapping with the given key and link.
    ///
    /// ## Errors
    ///
    /// This function will return an error if the given key is already in use.
    pub fn add_named(&mut self, key: String, link: String) -> Result<Entry, String> {
        let entry = Entry::from(link);
        // Update reverse hashmap
        match self.reverse_map.entry(entry.link.clone()) {
            hash_map::Entry::Occupied(mut e) => { 
                e.get_mut().push(key.clone()); 
            },
            hash_map::Entry::Vacant(e) => { 
                e.insert(vec![key.clone()]); 
            },
        }
        // Update forward hashmap
        if let hash_map::Entry::Vacant(e) = self.forward_map.entry(key) {
            e.insert(entry.clone());
            Ok(entry)
        } else {
            Err("Key already in use.".into())
        }
    }

    /// Remove the given mapping.
    /// 
    /// Returns `None` if the link alias does not exist.
    pub fn remove(&mut self, key: &str) -> Option<Entry> {
        let entry = self.forward_map.remove(key);
        
        // Update reverse hashmap
        if let Some(e) = entry {
            println!("map {:?}", self.reverse_map);
            let reverse = self.reverse_map.get_mut(&e.link)
                .expect("Missing reverse lookup entry (invalid state)");

            if reverse.len() == 1 {
                self.reverse_map.remove(&e.link);
            } else {
                let idx = reverse.iter().position(|x| *x == key)
                    .expect("Missing reverse lookup entry (invalid state)");
                reverse.remove(idx);
            }
            Some(e)
        } else {
            None
        }
    }

    /// Find aliases that map to the given link.
    /// 
    /// Returns `None` if the link has no associated aliases.
    pub fn find_by_link(&self, link: impl AsRef<str>) -> Option<&[String]> {
        self.reverse_map.get(link.as_ref()).map(|v| v.as_slice())
    }

    /// Save link data to the given file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String>{
        let path = path.as_ref();
        let data = toml::to_string(&self.forward_map.iter().collect::<HashMap<_, _>>())
            .unwrap();
        std::fs::write(path, data)
            .map_err(|e| format!("Could not write to file '{}': {}", path.display(), e))?;
        Ok(())
    }

    pub fn iter(&self) -> hash_map::Iter<'_, String, Entry> {
        self.forward_map.iter()
    }
}

impl IntoIterator for Links {
    type Item = (String, Entry);

    type IntoIter = std::collections::hash_map::IntoIter<String, Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.forward_map.into_iter()
    }
}


#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use vector_assertions::assert_vec_eq;

    use super::*;

    #[test]
    fn generate_key() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        let key = links.generate_key(link).unwrap();
        assert_eq!(key.len(), 4);
        let entry = links.add_named(key.clone(), link.to_string()).unwrap();
        let result = links.generate_key(link);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), (key, entry));
    }

    #[test]
    fn load_save() {
        let test_links = vec![
            ("key1", "https://example1.com"),
            ("key2", "https://example2.com"),
            ("ThisIsAVeryLongKeyWithManyManyCharacters", "https://example3.com"),
            ("PointsToSameURLAsKey1", "https://example1.com"),
            ("123456", "https://example4.com"),
            ("-_0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz", "https://example5.com"),
        ].into_iter()
        .map(|(k, v)| (k.to_string(), Entry::from(v.to_string())))
        .collect::<HashMap<_, _>>();

        let links = Links { forward_map: test_links, reverse_map: HashMap::new() };        
        let tmp_file = temp_dir().join("landmower_test.toml");
        
        links.save(&tmp_file).unwrap();

        let loaded = Links::load(&tmp_file).unwrap();            
        
        println!("{:?}", loaded);
        let old_keys: Vec<_> = links.forward_map        
            .keys()
            .collect();

        let new_keys: Vec<_> = loaded.forward_map
            .keys()            
            .collect();        
        let old_values: Vec<_> = links.forward_map
            .values()
            .map(|v| v.link.clone())
            .collect();        
        let new_values: Vec<_> = loaded.forward_map
            .values()
            .map(|v| v.link.clone())
            .collect();            

        assert_eq!(loaded.forward_map.len(), links.forward_map.len());        
        assert_vec_eq!(old_keys, new_keys);
        assert_vec_eq!(old_values, new_values);
    }

    #[test]
    fn add() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        
        let (key, entry) = links.add(link.to_string());
        
        assert_eq!(links.forward_map.len(), 1);
        assert_eq!(links.reverse_map.len(), 1);
        assert_eq!(links.reverse_map.get(&entry.link).unwrap().len(), 1);        
        assert_eq!(links.reverse_map.get(&entry.link).unwrap()[0], key);
    }

    #[test]
    fn add_named_base_case() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        let key = "key";

        let entry = links.add_named(key.to_string(), link.to_string()).unwrap();

        assert_eq!(links.forward_map.len(), 1);
        assert_eq!(links.reverse_map.len(), 1);
        assert_eq!(links.reverse_map.get(&entry.link).unwrap().len(), 1);
        assert_eq!(links.reverse_map.get(&entry.link).unwrap()[0], key);
    }

    #[test]
    fn add_named_key_collision() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        let key = "key";        
        links.add_named(key.to_string(), link.to_string()).unwrap();

        let result = links.add_named(key.to_string(), link.to_string());

        assert!(result.is_err());        
    }

    #[test]
    fn add_named_link_collision() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        let key1 = "key1";
        let key2 = "key2";
        let entry = links.add_named(key1.to_string(), link.to_string()).unwrap();        
        
        assert!(links.add_named(key2.to_string(), link.to_string()).is_ok());
        assert_eq!(links.reverse_map.get(&entry.link).unwrap().len(), 2);

        assert!(links.reverse_map.get(&entry.link).unwrap().contains(&key1.to_string()));
        assert!(links.reverse_map.get(&entry.link).unwrap().contains(&key2.to_string()));
    }

    #[test]
    fn remove() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        let key = "key";
        
        let entry = links.add_named(key.to_string(), link.to_string()).unwrap();
        let removed = links.remove(key).unwrap();

        assert_eq!(removed.link, entry.link);
        assert_eq!(links.forward_map.len(), 0);
        assert_eq!(links.reverse_map.len(), 0);
    }

    #[test]
    fn remove_nonexistent() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        let key = "key";

        links.add_named(key.to_string(), link.to_string()).unwrap();
        let removed = links.remove("nonexistent");
        
        assert!(removed.is_none());
    }

    #[test]
    fn find_by_link() {
        let mut links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let link = "https://example.com";
        let key1 = "key1";
        let key2 = "key2";
        links.add_named(key1.to_string(), link.to_string()).unwrap();
        links.add_named(key2.to_string(), link.to_string()).unwrap();
        
        let result = links.find_by_link(link);
        
        assert!(result.is_some());
        let result = result.unwrap();
        
        assert_eq!(result.len(), 2);
        assert!(result.contains(&key1.to_string()));
        assert!(result.contains(&key2.to_string()));
    }

    #[test]
    fn find_by_link_nonexistent() {
        let links = Links { forward_map: HashMap::new(), reverse_map: HashMap::new() };
        let result = links.find_by_link("nonexistent");

        assert!(result.is_none());
    }
}
use crate::{
    DelegationConditions, EventKind, Id, PublicKeyHex, SignatureHex, UncheckedUrl, Unixtime,
};
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};
#[cfg(feature = "speedy")]
use speedy::{Readable, Writable};
use std::fmt;

/// A tag on an Event
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "speedy", derive(Readable, Writable))]
pub enum Tag {
    /// Address 'a' tag to a parameterized replaceable event
    Address {
        /// EventKind
        kind: EventKind,

        /// Author
        pubkey: PublicKeyHex,

        /// d-tag identifier
        d: String,

        /// Relay URL
        relay_url: Option<UncheckedUrl>,
    },

    /// Content Warning to alert client to hide content until user approves
    ContentWarning(String),

    /// Delegation (Delegated Event Signing)
    Delegation {
        /// Public key of the delegator
        pubkey: PublicKeyHex,

        /// Conditions query string
        conditions: DelegationConditions,

        /// 64-byte schnorr signature of the sha256 hash of the delegation token
        sig: SignatureHex,
    },

    /// This is a reference to an event, where the first string is the event Id.
    /// The second string is defined in NIP-01 as an optional URL, but subsequent
    /// 'e' NIPs define more data and interpretations.
    Event {
        /// The Id of some other event that this event refers to
        id: Id,

        /// A recommended relay URL to find that other event
        recommended_relay_url: Option<UncheckedUrl>,

        /// A marker (commonly things like 'reply')
        marker: Option<String>,
    },

    /// A time when the event should be considered expired
    Expiration(Unixtime),

    /// 'p' This is a reference to a user by public key, where the first string is
    /// the PublicKey. The second string is defined in NIP-01 as an optional URL,
    /// but subsqeuent NIPs define more data and interpretations.
    Pubkey {
        /// The public key of the identity that this event refers to
        pubkey: PublicKeyHex,

        /// A recommended relay URL to find information on that public key
        recommended_relay_url: Option<UncheckedUrl>,

        /// A petname given to this identity by the event author
        petname: Option<String>,
    },

    /// 't' A hashtag
    Hashtag(String),

    /// 'r' A reference to a URL
    Reference {
        /// A relay url
        url: UncheckedUrl,

        /// An optional marker
        marker: Option<String>,
    },

    /// 'g' A geohash
    Geohash(String),

    /// 'd' Identifier tag
    Identifier(String),

    /// A subject. The first string is the subject. Should only be in TextNote events.
    Subject(String),

    /// A nonce tag for Proof of Work
    Nonce {
        /// A random number that makes the event hash meet the proof of work required
        nonce: String,

        /// The target number of bits for the proof of work
        target: Option<String>,
    },

    /// Parameter of a parameterized replaceable event
    Parameter(String),

    /// Title (30023 long form)
    Title(String),

    /// Any other tag
    Other {
        /// The tag name
        tag: String,

        /// The subsequent fields
        data: Vec<String>,
    },

    /// An empty array (kept so signature remains valid across ser/de)
    Empty,
}

impl Tag {
    /// Get the tag name for the tag (the first string in the array)a
    pub fn tagname(&self) -> String {
        match self {
            Tag::Address { .. } => "address".to_string(),
            Tag::ContentWarning(_) => "content-warning".to_string(),
            Tag::Delegation { .. } => "delegation".to_string(),
            Tag::Event { .. } => "e".to_string(),
            Tag::Expiration(_) => "expiration".to_string(),
            Tag::Pubkey { .. } => "p".to_string(),
            Tag::Hashtag(_) => "t".to_string(),
            Tag::Reference { .. } => "r".to_string(),
            Tag::Geohash(_) => "g".to_string(),
            Tag::Identifier(_) => "d".to_string(),
            Tag::Subject(_) => "subject".to_string(),
            Tag::Nonce { .. } => "nonce".to_string(),
            Tag::Parameter(_) => "parameter".to_string(),
            Tag::Title(_) => "title".to_string(),
            Tag::Other { tag, .. } => tag.clone(),
            Tag::Empty => panic!("empty tags have no tagname"),
        }
    }

    // Mock data for testing
    #[allow(dead_code)]
    pub(crate) fn mock() -> Tag {
        Tag::Event {
            id: Id::mock(),
            recommended_relay_url: Some(UncheckedUrl::mock()),
            marker: None,
        }
    }
}

impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Tag::Address {
                kind,
                pubkey,
                d,
                relay_url,
            } => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("a")?;
                let k: u32 = From::from(*kind);
                let s = format!("{}:{}:{}", k, pubkey, d);
                seq.serialize_element(&s)?;
                if let Some(ru) = relay_url {
                    seq.serialize_element(ru)?;
                }
                seq.end()
            }
            Tag::ContentWarning(msg) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("content-warning")?;
                seq.serialize_element(msg)?;
                seq.end()
            }
            Tag::Delegation {
                pubkey,
                conditions,
                sig,
            } => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("delegation")?;
                seq.serialize_element(pubkey)?;
                seq.serialize_element(conditions)?;
                seq.serialize_element(sig)?;
                seq.end()
            }
            Tag::Event {
                id,
                recommended_relay_url,
                marker,
            } => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("e")?;
                seq.serialize_element(id)?;
                if let Some(rru) = recommended_relay_url {
                    seq.serialize_element(rru)?;
                } else if marker.is_some() {
                    seq.serialize_element("")?;
                }
                if let Some(m) = marker {
                    seq.serialize_element(m)?;
                }
                seq.end()
            }
            Tag::Expiration(time) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("expiration")?;
                seq.serialize_element(time)?;
                seq.end()
            }
            Tag::Pubkey {
                pubkey,
                recommended_relay_url,
                petname,
            } => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("p")?;
                seq.serialize_element(pubkey)?;
                if let Some(rru) = recommended_relay_url {
                    seq.serialize_element(rru)?;
                } else if petname.is_some() {
                    seq.serialize_element("")?;
                }
                if let Some(pn) = petname {
                    seq.serialize_element(pn)?;
                }
                seq.end()
            }
            Tag::Hashtag(hashtag) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("t")?;
                seq.serialize_element(hashtag)?;
                seq.end()
            }
            Tag::Reference { url, marker } => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("r")?;
                seq.serialize_element(url)?;
                if let Some(m) = marker {
                    seq.serialize_element(m)?
                }
                seq.end()
            }
            Tag::Geohash(geohash) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("g")?;
                seq.serialize_element(geohash)?;
                seq.end()
            }
            Tag::Identifier(id) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("d")?;
                seq.serialize_element(id)?;
                seq.end()
            }
            Tag::Subject(subject) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("subject")?;
                seq.serialize_element(subject)?;
                seq.end()
            }
            Tag::Nonce { nonce, target } => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("nonce")?;
                seq.serialize_element(nonce)?;
                if let Some(t) = target {
                    seq.serialize_element(t)?;
                }
                seq.end()
            }
            Tag::Parameter(parameter) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("parameter")?;
                seq.serialize_element(parameter)?;
                seq.end()
            }
            Tag::Title(title) => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element("title")?;
                seq.serialize_element(title)?;
                seq.end()
            }
            Tag::Other { tag, data } => {
                let mut seq = serializer.serialize_seq(None)?;
                seq.serialize_element(tag)?;
                for s in data.iter() {
                    seq.serialize_element(s)?;
                }
                seq.end()
            }
            Tag::Empty => {
                let seq = serializer.serialize_seq(Some(0))?;
                seq.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(TagVisitor)
    }
}

struct TagVisitor;

impl<'de> Visitor<'de> for TagVisitor {
    type Value = Tag;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a sequence of strings")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Tag, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let tagname: &str = match seq.next_element()? {
            Some(e) => e,
            None => return Ok(Tag::Empty),
        };
        if tagname == "a" {
            if let Some(a) = seq.next_element::<&str>()? {
                let relay_url: Option<UncheckedUrl> = seq.next_element()?;
                let failvec = match relay_url {
                    Some(ref url) => vec![a.to_string(), url.as_str().to_owned()],
                    None => vec![a.to_string()],
                };

                let parts: Vec<&str> = a.split(':').collect();
                if parts.len() != 3 {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: failvec,
                    });
                }
                let kindnum: u32 = match parts[0].parse::<u32>() {
                    Ok(u) => u,
                    Err(_) => {
                        return Ok(Tag::Other {
                            tag: tagname.to_string(),
                            data: failvec,
                        });
                    }
                };
                let kind: EventKind = From::from(kindnum);
                let pubkey: PublicKeyHex = match PublicKeyHex::try_from_str(parts[1]) {
                    Ok(pk) => pk,
                    Err(_) => {
                        return Ok(Tag::Other {
                            tag: tagname.to_string(),
                            data: failvec,
                        });
                    }
                };
                Ok(Tag::Address {
                    kind,
                    pubkey,
                    d: parts[2].to_string(),
                    relay_url,
                })
            } else {
                Ok(Tag::Other {
                    tag: tagname.to_string(),
                    data: vec![],
                })
            }
        } else if tagname == "content-warning" {
            let msg = match seq.next_element()? {
                Some(s) => s,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            Ok(Tag::ContentWarning(msg))
        } else if tagname == "delegation" {
            let pubkey: PublicKeyHex = match seq.next_element()? {
                Some(pk) => pk,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            let conditions: DelegationConditions = match seq.next_element()? {
                Some(c) => c,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![pubkey.into_string()],
                    });
                }
            };
            let sig: SignatureHex = match seq.next_element()? {
                Some(s) => s,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![pubkey.into_string(), conditions.as_string()],
                    });
                }
            };
            Ok(Tag::Delegation {
                pubkey,
                conditions,
                sig,
            })
        } else if tagname == "e" {
            let id: Id = match seq.next_element()? {
                Some(id) => id,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            let recommended_relay_url: Option<UncheckedUrl> = seq.next_element()?;
            let marker: Option<String> = seq.next_element()?;
            Ok(Tag::Event {
                id,
                recommended_relay_url,
                marker,
            })
        } else if tagname == "expiration" {
            let time = match seq.next_element()? {
                Some(t) => t,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            Ok(Tag::Expiration(time))
        } else if tagname == "p" {
            let pubkey: PublicKeyHex = match seq.next_element()? {
                Some(p) => p,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            let recommended_relay_url: Option<UncheckedUrl> = seq.next_element()?;
            let petname: Option<String> = seq.next_element()?;
            Ok(Tag::Pubkey {
                pubkey,
                recommended_relay_url,
                petname,
            })
        } else if tagname == "t" {
            let tag = match seq.next_element()? {
                Some(t) => t,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            Ok(Tag::Hashtag(tag))
        } else if tagname == "r" {
            let refr: UncheckedUrl = match seq.next_element()? {
                Some(r) => r,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            let marker: Option<String> = seq.next_element()?;
            Ok(Tag::Reference { url: refr, marker })
        } else if tagname == "g" {
            let geo = match seq.next_element()? {
                Some(g) => g,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            Ok(Tag::Geohash(geo))
        } else if tagname == "d" {
            let id = match seq.next_element()? {
                Some(id) => id,
                None => {
                    // Implicit empty value
                    return Ok(Tag::Identifier("".to_string()));
                }
            };
            Ok(Tag::Identifier(id))
        } else if tagname == "subject" {
            let sub = match seq.next_element()? {
                Some(s) => s,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            Ok(Tag::Subject(sub))
        } else if tagname == "nonce" {
            let nonce = match seq.next_element()? {
                Some(n) => n,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            let target: Option<String> = seq.next_element()?;
            Ok(Tag::Nonce { nonce, target })
        } else if tagname == "parameter" {
            let param = match seq.next_element()? {
                Some(s) => s,
                None => "".to_owned(), // implicit parameter
            };
            Ok(Tag::Parameter(param))
        } else if tagname == "title" {
            let title = match seq.next_element()? {
                Some(s) => s,
                None => {
                    return Ok(Tag::Other {
                        tag: tagname.to_string(),
                        data: vec![],
                    });
                }
            };
            Ok(Tag::Title(title))
        } else {
            let mut data = Vec::new();
            loop {
                match seq.next_element()? {
                    None => {
                        return Ok(Tag::Other {
                            tag: tagname.to_string(),
                            data,
                        })
                    }
                    Some(s) => data.push(s),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    test_serde! {Tag, test_tag_serde}

    #[test]
    fn test_a_tag() {
        let tag = Tag::Address {
            kind: EventKind::LongFormContent,
            pubkey: PublicKeyHex::mock_deterministic(),
            d: "Testing123".to_owned(),
            relay_url: Some(UncheckedUrl("wss://relay.mikedilger.com/".to_string())),
        };
        let string = serde_json::to_string(&tag).unwrap();
        let tag2 = serde_json::from_str(&string).unwrap();
        assert_eq!(tag, tag2);

        let tag = Tag::Address {
            kind: EventKind::LongFormContent,
            pubkey: PublicKeyHex::mock_deterministic(),
            d: "Testing123".to_owned(),
            relay_url: None,
        };
        let string = serde_json::to_string(&tag).unwrap();
        let tag2 = serde_json::from_str(&string).unwrap();
        assert_eq!(tag, tag2);
    }
}

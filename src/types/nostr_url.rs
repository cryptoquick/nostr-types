use super::{EventAddr, EventPointer, Id, Profile, PublicKey, UncheckedUrl};
use crate::Error;
use bech32::{FromBase32, ToBase32};
use lazy_static::lazy_static;

/// A bech32 sequence representing a nostr object (or set of objects)
// note, internally we store them as the object the sequence represents
#[derive(Clone, Debug)]
pub enum NostrBech32 {
    /// naddr - a NostrBech32 parameterized replaceable event coordinate
    EventAddr(EventAddr),
    /// nevent - a NostrBech32 representing an event and a set of relay URLs
    EventPointer(EventPointer),
    /// note - a NostrBech32 representing an event
    Id(Id),
    /// nprofile - a NostrBech32 representing a public key and a set of relay URLs
    Profile(Profile),
    /// npub - a NostrBech32 representing a public key
    Pubkey(PublicKey),
    /// nrelay - a NostrBech32 representing a set of relay URLs
    Relay(UncheckedUrl),
}

impl std::fmt::Display for NostrBech32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            NostrBech32::EventAddr(ea) => write!(f, "{}", ea.as_bech32_string()),
            NostrBech32::EventPointer(ep) => write!(f, "{}", ep.as_bech32_string()),
            NostrBech32::Id(i) => write!(f, "{}", i.as_bech32_string()),
            NostrBech32::Profile(p) => write!(f, "{}", p.as_bech32_string()),
            NostrBech32::Pubkey(pk) => write!(f, "{}", pk.as_bech32_string()),
            NostrBech32::Relay(url) => write!(f, "{}", Self::nrelay_as_bech32_string(url)),
        }
    }
}

impl NostrBech32 {
    /// Create from a `PublicKey`
    pub fn new_pubkey(pubkey: PublicKey) -> NostrBech32 {
        NostrBech32::Pubkey(pubkey)
    }

    /// Create from a `Profile`
    pub fn new_profile(profile: Profile) -> NostrBech32 {
        NostrBech32::Profile(profile)
    }

    /// Create from an `Id`
    pub fn new_id(id: Id) -> NostrBech32 {
        NostrBech32::Id(id)
    }

    /// Create from an `EventPointer`
    pub fn new_event_pointer(ep: EventPointer) -> NostrBech32 {
        NostrBech32::EventPointer(ep)
    }

    /// Create from an `UncheckedUrl`
    pub fn new_relay(url: UncheckedUrl) -> NostrBech32 {
        NostrBech32::Relay(url)
    }

    /// Try to convert a string into a NostrBech32. Must not have leading or trailing
    /// junk for this to work.
    pub fn try_from_string(s: &str) -> Option<NostrBech32> {
        if s.get(..6) == Some("naddr1") {
            if let Ok(ea) = EventAddr::try_from_bech32_string(s) {
                return Some(NostrBech32::EventAddr(ea));
            }
        } else if s.get(..7) == Some("nevent1") {
            if let Ok(ep) = EventPointer::try_from_bech32_string(s) {
                return Some(NostrBech32::EventPointer(ep));
            }
        } else if s.get(..5) == Some("note1") {
            if let Ok(id) = Id::try_from_bech32_string(s) {
                return Some(NostrBech32::Id(id));
            }
        } else if s.get(..9) == Some("nprofile1") {
            if let Ok(p) = Profile::try_from_bech32_string(s) {
                return Some(NostrBech32::Profile(p));
            }
        } else if s.get(..5) == Some("npub1") {
            if let Ok(pk) = PublicKey::try_from_bech32_string(s) {
                return Some(NostrBech32::Pubkey(pk));
            }
        } else if s.get(..7) == Some("nrelay1") {
            if let Ok(urls) = Self::nrelay_try_from_bech32_string(s) {
                return Some(NostrBech32::Relay(urls));
            }
        }
        None
    }

    /// Find all `NostrBech32`s in a string, returned in the order found
    pub fn find_all_in_string(s: &str) -> Vec<NostrBech32> {
        let mut output: Vec<NostrBech32> = Vec::new();
        let mut cursor = 0;
        while let Some((relstart, relend)) = find_nostr_bech32_pos(s.get(cursor..).unwrap()) {
            if let Some(nurl) =
                NostrBech32::try_from_string(s.get(cursor + relstart..cursor + relend).unwrap())
            {
                output.push(nurl);
            }
            cursor += relend;
        }
        output
    }

    // Because nrelay uses TLV, we can't just use UncheckedUrl::as_bech32_string()
    fn nrelay_as_bech32_string(url: &UncheckedUrl) -> String {
        let mut tlv: Vec<u8> = Vec::new();
        tlv.push(0); // special for nrelay
        tlv.push(url.0.len() as u8); // length
        tlv.extend(url.0.as_bytes());
        bech32::encode("nrelay", tlv.to_base32(), bech32::Variant::Bech32).unwrap()
    }

    // Because nrelay uses TLV, we can't just use UncheckedUrl::try_from_bech32_string
    fn nrelay_try_from_bech32_string(s: &str) -> Result<UncheckedUrl, Error> {
        let data = bech32::decode(s)?;
        if data.0 != "nrelay" {
            Err(Error::WrongBech32("nrelay".to_string(), data.0))
        } else {
            let mut url: Option<UncheckedUrl> = None;
            let tlv = Vec::<u8>::from_base32(&data.1)?;
            let mut pos = 0;
            loop {
                // we need at least 2 more characters for anything meaningful
                if pos > tlv.len() - 2 {
                    break;
                }
                let ty = tlv[pos];
                let len = tlv[pos + 1] as usize;
                pos += 2;
                if pos + len > tlv.len() {
                    return Err(Error::InvalidUrlTlv);
                }
                let raw = &tlv[pos..pos + len];
                #[allow(clippy::single_match)]
                match ty {
                    0 => {
                        let relay_str = std::str::from_utf8(raw)?;
                        let relay = UncheckedUrl::from_str(relay_str);
                        url = Some(relay);
                    }
                    _ => {} // unhandled type for nrelay
                }
                pos += len;
            }
            if let Some(url) = url {
                Ok(url)
            } else {
                Err(Error::InvalidUrlTlv)
            }
        }
    }
}

/// A Nostr URL (starting with 'nostr:')
#[derive(Clone, Debug)]
pub struct NostrUrl(pub NostrBech32);

impl std::fmt::Display for NostrUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "nostr:")?;
        self.0.fmt(f)
    }
}

impl NostrUrl {
    /// Create a new NostrUrl from a NostrBech32
    pub fn new(bech32: NostrBech32) -> NostrUrl {
        NostrUrl(bech32)
    }

    /// Try to convert a string into a NostrUrl. Must not have leading or trailing
    /// junk for this to work.
    pub fn try_from_string(s: &str) -> Option<NostrUrl> {
        if s.get(..6) != Some("nostr:") {
            return None;
        }
        NostrBech32::try_from_string(s.get(6..).unwrap()).map(NostrUrl)
    }

    /// Find all `NostrUrl`s in a string, returned in the order found
    /// (If not prefixed with 'nostr:' they will not count, see NostrBech32)
    pub fn find_all_in_string(s: &str) -> Vec<NostrUrl> {
        let mut output: Vec<NostrUrl> = Vec::new();
        let mut cursor = 0;
        while let Some((relstart, relend)) = find_nostr_url_pos(s.get(cursor..).unwrap()) {
            if let Some(nurl) =
                NostrUrl::try_from_string(s.get(cursor + relstart..cursor + relend).unwrap())
            {
                output.push(nurl);
            }
            cursor += relend;
        }
        output
    }

    /// This converts all recognized bech32 sequences into proper nostr URLs by adding
    /// the "nostr:" prefix where missing.
    pub fn urlize(s: &str) -> String {
        let mut output: String = String::with_capacity(s.len());
        let mut cursor = 0;
        while let Some((relstart, relend)) = find_nostr_bech32_pos(s.get(cursor..).unwrap()) {
            // If it already has it, leave it alone
            if relstart >= 6 && s.get(cursor + relstart - 6..cursor + relstart) == Some("nostr:") {
                output.push_str(s.get(cursor..cursor + relend).unwrap());
            } else {
                output.push_str(s.get(cursor..cursor + relstart).unwrap());
                output.push_str("nostr:");
                output.push_str(s.get(cursor + relstart..cursor + relend).unwrap());
            }
            cursor += relend;
        }
        output.push_str(s.get(cursor..).unwrap());
        output
    }
}

/// Returns start and end position of next valid NostrBech32
pub fn find_nostr_bech32_pos(s: &str) -> Option<(usize, usize)> {
    // BECH32 Alphabet:
    // qpzry9x8gf2tvdw0s3jn54khce6mua7l
    // acdefghjklmnpqrstuvwxyz023456789
    use regex::Regex;
    lazy_static! {
        static ref BECH32_RE: Regex = Regex::new(
            r#"(?:^|[^a-zA-Z0-9])((?:nsec|npub|nprofile|note|nevent|nrelay|naddr)1[ac-hj-np-z02-9]{7,})(?:$|[^a-zA-Z0-9])"#
        ).expect("Could not compile nostr URL regex");
    }
    BECH32_RE.captures(s).map(|cap| {
        let mat = cap.get(1).unwrap();
        (mat.start(), mat.end())
    })
}

/// Returns start and end position of next valid NostrUrl
pub fn find_nostr_url_pos(s: &str) -> Option<(usize, usize)> {
    // BECH32 Alphabet:
    // qpzry9x8gf2tvdw0s3jn54khce6mua7l
    // acdefghjklmnpqrstuvwxyz023456789
    use regex::Regex;
    lazy_static! {
        static ref NOSTRURL_RE: Regex = Regex::new(
            r#"(?:^|[^a-zA-Z0-9])(nostr:(?:nsec|npub|nprofile|note|nevent|nrelay|naddr)1[ac-hj-np-z02-9]{7,})(?:$|[^a-zA-Z0-9])"#
        ).expect("Could not compile nostr URL regex");
    }
    NOSTRURL_RE.captures(s).map(|cap| {
        let mat = cap.get(1).unwrap();
        (mat.start(), mat.end())
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_nostr_bech32_try_from_string() {
        let a = "npub1sn0wdenkukak0d9dfczzeacvhkrgz92ak56egt7vdgzn8pv2wfqqhrjdv9";
        let nurl = NostrBech32::try_from_string(a).unwrap();
        assert!(matches!(nurl, NostrBech32::Pubkey(..)));

        let b = "nprofile1qqsrhuxx8l9ex335q7he0f09aej04zpazpl0ne2cgukyawd24mayt8gpp4mhxue69uhhytnc9e3k7mgpz4mhxue69uhkg6nzv9ejuumpv34kytnrdaksjlyr9p";
        let nurl = NostrBech32::try_from_string(b).unwrap();
        assert!(matches!(nurl, NostrBech32::Profile(..)));

        let c = "note1fntxtkcy9pjwucqwa9mddn7v03wwwsu9j330jj350nvhpky2tuaspk6nqc";
        let nurl = NostrBech32::try_from_string(c).unwrap();
        assert!(matches!(nurl, NostrBech32::Id(..)));

        let d = "nevent1qqstna2yrezu5wghjvswqqculvvwxsrcvu7uc0f78gan4xqhvz49d9spr3mhxue69uhkummnw3ez6un9d3shjtn4de6x2argwghx6egpr4mhxue69uhkummnw3ez6ur4vgh8wetvd3hhyer9wghxuet5nxnepm";
        let nurl = NostrBech32::try_from_string(d).unwrap();
        assert!(matches!(nurl, NostrBech32::EventPointer(..)));

        let e = "naddr1qqxk67txd9e8xardv96x7mt9qgsgfvxyd2mfntp4avk29pj8pwz7pqwmyzrummmrjv3rdsuhg9mc9agrqsqqqa28rkfdwv";
        let nurl = NostrBech32::try_from_string(e).unwrap();
        assert!(matches!(nurl, NostrBech32::EventAddr(..)));

        let f = "naddr1qq9xuum9vd382mntv4eqz8nhwden5te0dehhxarj9eek2argvehhyurjd9mxzcme9e3k7mgpzamhxue69uhhyetvv9ujucm4wfex2mn59en8j6gpzfmhxue69uhhqatjwpkx2urpvuhx2ucpr9mhxue69uhkummnw3ezu7n9vfjkget99e3kcmm4vsq32amnwvaz7tm9v3jkutnwdaehgu3wd3skueqpp4mhxue69uhkummn9ekx7mqpr9mhxue69uhhqatjv9mxjerp9ehx7um5wghxcctwvsq3samnwvaz7tmjv4kxz7fwwdhx7un59eek7cmfv9kqz9rhwden5te0wfjkccte9ejxzmt4wvhxjmcpr4mhxue69uhkummnw3ezu6r0wa6x7cnfw33k76tw9eeksmmsqy2hwumn8ghj7mn0wd68ytn2v96x6tnvd9hxkqgkwaehxw309ashgmrpwvhxummnw3ezumrpdejqzynhwden5te0danxvcmgv95kutnsw43qzynhwden5te0wfjkccte9enrw73wd9hsz9rhwden5te0wfjkccte9ehx7um5wghxyecpzemhxue69uhhyetvv9ujumn0wd68ytnfdenx7qg7waehxw309ahx7um5wgkhyetvv9ujumn0ddhhgctjduhxxmmdqy28wumn8ghj7cnvv9ehgu3wvcmh5tnc09aqzymhwden5te0wfjkcctev93xcefwdaexwqgcwaehxw309akxjemgw3hxjmn8wfjkccte9e3k7mgprfmhxue69uhhyetvv9ujumn0wd68y6trdpjhxtn0wfnszyrhwden5te0dehhxarj9emkjmn9qyrkxmmjv93kcegzypl4c26wfzswnlk2vwjxky7dhqjgnaqzqwvdvz3qwz5k3j4grrt46qcyqqq823cd90lu6";
        let nurl = NostrBech32::try_from_string(f).unwrap();
        assert!(matches!(nurl, NostrBech32::EventAddr(..)));

        let g = "nrelay1qqghwumn8ghj7mn0wd68yv339e3k7mgftj9ag";
        let nurl = NostrBech32::try_from_string(g).unwrap();
        assert!(matches!(nurl, NostrBech32::Relay(..)));

        // too short
        let short = "npub1sn0wdenkukak0d9dfczzeacvhkrgz92ak56egt7vdgzn8pv2wfqqhrjdv";
        assert!(NostrBech32::try_from_string(short).is_none());

        // bad char
        let badchar = "note1fntxtkcy9pjwucqwa9mddn7v03wwwsu9j330jj350nvhpky2tuaspk6bqc";
        assert!(NostrBech32::try_from_string(badchar).is_none());

        // unknown prefix char
        let unknown = "nurl1sn0wdenkukak0d9dfczzeacvhkrgz92ak56egt7vdgzn8pv2wfqqhrjdv9";
        assert!(NostrBech32::try_from_string(unknown).is_none());
    }

    #[test]
    fn test_nostr_urlize() {
        let sample = r#"This is now the offical Gossip Client account.  Please follow it.  I will be reposting it's messages for some time until it catches on.

nprofile1qqsrjerj9rhamu30sjnuudk3zxeh3njl852mssqng7z4up9jfj8yupqpzamhxue69uhhyetvv9ujumn0wd68ytnfdenx7tcpz4mhxue69uhkummnw3ezummcw3ezuer9wchszxmhwden5te0dehhxarj9ekkj6m9v35kcem9wghxxmmd9uq3xamnwvaz7tm0venxx6rpd9hzuur4vghsz8nhwden5te0dehhxarj94c82c3wwajkcmr0wfjx2u3wdejhgtcsfx2xk

#[1]
"#;
        let fixed = NostrUrl::urlize(sample);
        println!("{fixed}");
        assert!(fixed.contains("nostr:nprofile1"));

        let sample2 = r#"Have you been switching nostr clients lately?
Could be related to:
nostr:note10ttnuuvcs29y3k23gwrcurw2ksvgd7c2rrqlfx7urmt5m963vhss8nja90
"#;
        let nochange = NostrUrl::urlize(sample2);
        assert_eq!(sample2.len(), nochange.len());

        let sample3 = r#"Have you been switching nostr clients lately?
Could be related to:
note10ttnuuvcs29y3k23gwrcurw2ksvgd7c2rrqlfx7urmt5m963vhss8nja90
"#;
        let fixed = NostrUrl::urlize(sample3);
        assert!(fixed.contains("nostr:note1"));
        assert!(fixed.len() > sample3.len());
    }

    #[test]
    fn test_nostr_url_unicode_issues() {
        let sample = r#"🌝🐸note1fntxtkcy9pjwucqwa9mddn7v03wwwsu9j330jj350nvhpky2tuaspk6nqc"#;
        assert!(NostrUrl::try_from_string(sample).is_none())
    }

    #[test]
    fn test_multiple_nostr_urls() {
        let sample = r#"
Here is a list of relays I use and consider reliable so far. I've included some relevant information for each relay such as if payment is required or [NIP-33](https://nips.be/33) is supported. I'll be updating this list as I discover more good relays, which ones do you find reliable?

## Nokotaro

nostr:nrelay1qq0hwumn8ghj7mn0wd68yttjv4kxz7fwdehkkmm5v9ex7tnrdakj78zlgae

- Paid? **No**
- [NIP-33](https://nips.be/33) supported? **Yes**
- Operator: nostr:npub12ftld459xqw7s7fqnxstzu7r74l5yagxztwcwmaqj4d24jgpj2csee3mx0

## Nostr World

nostr:nrelay1qqvhwumn8ghj7mn0wd68ytthdaexcepwdqeh5tn2wqhsv5kg7j

- Paid? **Yes**
- [NIP-33](https://nips.be/33) supported? **Yes**
- Operator: nostr:npub1zpq2gsz25wsgun2e4gtks9p63j7fvyfd46weyjzp5tv6yys89zcsjdflcv

## Nos.lol

nostr:nrelay1qq88wumn8ghj7mn0wvhxcmmv9uvj5a67

- Paid? **No**
- [NIP-33](https://nips.be/33) supported? **No**
- Operator: nostr:npub1nlk894teh248w2heuu0x8z6jjg2hyxkwdc8cxgrjtm9lnamlskcsghjm9c

## Nostr Wine

nostr:nrelay1qqghwumn8ghj7mn0wd68ytnhd9hx2tcw2qslz

- Paid? **Yes**
- [NIP-33](https://nips.be/33) supported? **No**
- Operators: nostr:npub1qlkwmzmrhzpuak7c2g9akvcrh7wzkd7zc7fpefw9najwpau662nqealf5y & nostr:npub18kzz4lkdtc5n729kvfunxuz287uvu9f64ywhjz43ra482t2y5sks0mx5sz

## Nostrich Land

nostr:nrelay1qqvhwumn8ghj7un9d3shjtnwdaehgunfvd5zumrpdejqpdl8ln

- Paid? **Yes**
- [NIP-33](https://nips.be/33) supported? **No**
- Operator: nostr:nprofile1qqsxf8h0u35dmvg8cp0t5mg9z8f222v9grly6hcqw2cqvdsq3lrjlyspr9mhxue69uhhyetvv9ujumn0wd68y6trdqhxcctwvsj9ulqc
"#;

        assert_eq!(NostrUrl::find_all_in_string(sample).len(), 11);
    }

    #[test]
    fn test_generate_nrelay() {
        let url = UncheckedUrl("wss://nostr.mikedilger.com/".to_owned());
        let nb32 = NostrBech32::new_relay(url);
        let nurl = NostrUrl(nb32);
        println!("{}", nurl);
    }
}

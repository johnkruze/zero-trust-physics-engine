//! Who this is. `initialize` and `server/discover` carry the verifier's name,
//! title, version and site in `serverInfo`, and in `instructions` what it is,
//! where to start, where a model stops and which tools check a proof. `cite`
//! gives the service's citation (its maker, site, address and paper), and
//! `find_model` gives the way to ask for a model when nothing served fits.
//! Until 0.14.2 the instructions also named the maker, how to reach him, the
//! site, the paper and the two sources.

/// The verifier's name: `serverInfo.name`.
pub const NAME: &str = "Zero-Trust Physics";
/// `serverInfo.title`: the name and what it is.
pub const TITLE: &str = "Zero-Trust Physics: a physics verifier for Super Intelligence";
/// Who made it.
pub const MAKER: &str = "John Kruze";
pub const COMPANY: &str = "Protocol Company";
/// Where the story is, for a person and for an agent.
pub const SITE: &str = "https://zerotrustphysics.com";
pub const SITE_FOR_AGENTS: &str = "https://zerotrustphysics.com/llms.txt";
/// The paper.
pub const PAPER: &str = "https://doi.org/10.5281/zenodo.23305697";
/// The engine's own source.
pub const ENGINE_SOURCE: &str = "https://github.com/johnkruze/zero-trust-physics-engine";
/// The programs that confirm a cryptographic proof with no engine.
pub const CONFIRMING_PROGRAMS: &str = "https://github.com/johnkruze/zero-trust-physics";
/// The open address the verifier is served at.
pub const ADDRESS: &str = "https://mcp.zerotrustphysics.com/mcp";
/// How to reach John Kruze.
pub const EMAIL: &str = "kruze@zerotrustphysics.com";
pub const PHONE: &str = "928-264-7883";

/// The most characters the instructions and each tool's description come to
/// on any connection. A client may show less than it is sent: Claude Code
/// cuts each at 2,048 characters. What a description has no room for is said
/// in the tool's input schema, at the input it is about.
pub const MOST_CHARS_SAID: usize = 2_000;

/// `instructions`, since 0.14.2 (John Kruze, 18 October 2026): what this
/// is, where to start, what a model does past what it was compared with and
/// where it stops, and which tools check a proof. Who made it, the site, the
/// paper and the sources are not repeated here: an agent already connected
/// asks `cite` for them. Until 0.14.2 the instructions were an introduction
/// of five sentences (the maker, his telephone, the site, the paper and two
/// sources) and a paragraph of what each tool is for, 1,417 characters; each
/// tool's description says what it does. The sentence of where a model
/// stops is John Kruze's of 17 October 2026, word for word, as the site and
/// Hugging Face say it. "check any proof" in the words approved was said as
/// "check the proof of a run, a solution or a dataset":
/// a step's `sha256_seal` is checked by computing the step again, and a
/// comparison's seal by `genesis_seal verify-validation`.
pub const INSTRUCTIONS: &str = "Zero-Trust Physics: deterministic physics, a proof on every answer. Start with find_model (describe the real thing) or list_cards / list_solvers, and read describe_card or describe_solver (form \"short\") before a first request. Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason. verify_tape, verify_solution and verify_dataset check the proof of a run, a solution or a dataset; cite gives sources.";

/// `instructions`, as `initialize` and `server/discover` carry them.
pub fn instructions() -> String {
    INSTRUCTIONS.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words John Kruze approved on 18 October 2026, with "check any
    /// proof" said of the three proofs those tools check; nothing of the
    /// maker's, which `cite` gives.
    #[test]
    fn the_instructions_are_the_words_approved_and_name_served_tools() {
        let said = instructions();
        assert!(said.starts_with("Zero-Trust Physics: deterministic physics, a proof on every answer."), "{said}");
        assert!(said.starts_with(NAME));
        for held in [
            "Start with find_model (describe the real thing) or list_cards / list_solvers, and read describe_card or describe_solver (form \"short\") before a first request.",
            "Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason.",
            "verify_tape, verify_solution and verify_dataset check the proof of a run, a solution or a dataset; cite gives sources.",
        ] {
            assert!(said.contains(held), "{held}: {said}");
        }
        for gone in [MAKER, COMPANY, SITE, PAPER, ENGINE_SOURCE, CONFIRMING_PROGRAMS, EMAIL, PHONE, "mathematical or physical limit", "caller", "AI ", "gateway", "genesis"] {
            assert!(!said.contains(gone), "{gone}: {said}");
        }
        let served: Vec<String> =
            crate::mcp_server::tools_in(&crate::view::View::full()).into_iter().map(|tool| tool.name.to_string()).collect();
        let named: Vec<&str> = said.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).filter(|word| word.contains('_')).collect();
        assert_eq!(named.len(), 8, "{named:?}");
        for word in named {
            assert!(served.iter().any(|name| name == word), "{word} is named and is not a tool");
        }
        assert!(said.contains(" cite "));
        assert!(served.iter().any(|name| name == "cite"));
        assert_eq!(said.chars().count(), said.len());
        assert_eq!(said.len(), 563);
        assert_eq!(PAPER, crate::cite::PAPER_DOI_URL);
    }
}

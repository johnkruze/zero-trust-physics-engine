//! Who this is. The first thing an agent reads from any connection names the
//! verifier and its maker, and says where the story, the paper, the engine's
//! source and the programs that confirm a proof are, and how to reach the
//! person who made it. `initialize` and `server/discover` carry it in
//! `serverInfo` and at the head of `instructions`; `cite` gives it as a
//! citation of the service; and `find_model` gives the way to ask for a model
//! when nothing served fits.

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

/// What the tools are for, each sentence a statement of what is so: which
/// tools name what is served, which describe a model and in which form,
/// which compute with a proof, and which check one. Since 0.14.1 the
/// sentence of the four that compute no longer ends "each with its proof":
/// the introduction says every answer comes with one.
pub const WORKING_INSTRUCTIONS: &str = "list_cards and list_solvers name every model served, and find_model finds one from a plain description of a real thing. describe_card and describe_solver give a model's inputs and limits: form \"short\" is a page enough for a first request, and the full form, the default, explains a refusal and is the one a result rests on. compute_step computes a step, run_trajectory a run, solve a solution and run_dataset a plan of them. verify_tape, verify_solution and verify_dataset verify a seal: the short form, request and seal, shows it was computed as sealed; the long form, the whole answer, recomputes it: every byte is the engine's.";

/// What this is, what a model does past what it was compared with and where
/// it stops, who made it and where to read more: five sentences.
pub fn introduction() -> String {
    format!(
        "This is {NAME}, a physics verifier for Super Intelligence: every answer comes with a cryptographic proof. Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason. It was made by {MAKER}, {COMPANY}. The story is at {SITE} (for an agent, {SITE_FOR_AGENTS}), the paper is {PAPER}, the engine's source is at {ENGINE_SOURCE}, the programs that confirm a proof with no engine are at {CONFIRMING_PROGRAMS}, and he is reached at {EMAIL} or {PHONE}."
    )
}

/// `instructions`: the introduction, then what the tools are for.
pub fn instructions() -> String {
    format!("{} {WORKING_INSTRUCTIONS}", introduction())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_introduction_names_the_thing_its_maker_and_where_to_read_more() {
        let said = introduction();
        assert!(said.starts_with("This is Zero-Trust Physics, a physics verifier for Super Intelligence:"), "{said}");
        for held in [
            "every answer comes with a cryptographic proof. Past what a model was compared with, it keeps going with the same equations and flags every step. It stops only at a physical limit, or where its equations, constants or tables no longer hold, and it names the reason. It was made by",
            "John Kruze, Protocol Company",
            "https://zerotrustphysics.com",
            "https://zerotrustphysics.com/llms.txt",
            "https://doi.org/10.5281/zenodo.23305697",
            "the engine's source is at https://github.com/johnkruze/zero-trust-physics-engine,",
            "the programs that confirm a proof with no engine are at https://github.com/johnkruze/zero-trust-physics,",
            "and he is reached at kruze@zerotrustphysics.com",
            "928-264-7883",
        ] {
            assert!(said.contains(held), "{held}: {said}");
        }
        // Five plain sentences, and the words the company does not use of
        // itself are not in them. Since 0.14.1 the second and third are John
        // Kruze's own, as the site, Hugging Face and GitHub say them: a model
        // keeps going past what it was compared with, flagging every step, and
        // stops where its physics, equations, constants or tables end, where
        // until 0.14.1 it was said to stop at a mathematical or physical limit.
        assert_eq!(said.matches(". ").count() + 1, 5, "{said}");
        assert!(!said.contains("mathematical or physical limit"), "{said}");
        for word in ["caller", "AI ", "gateway", "genesis"] {
            assert!(!said.contains(word), "{word}: {said}");
        }
        assert!(instructions().ends_with(WORKING_INSTRUCTIONS));
        assert_eq!(PAPER, crate::cite::PAPER_DOI_URL);
    }

    /// What the tools are for is said in statements: no sentence opens with
    /// a verb telling the agent what to do, and every tool it names is served.
    #[test]
    fn the_working_words_are_statements_and_name_served_tools() {
        for sentence in WORKING_INSTRUCTIONS.split(". ") {
            let first = sentence.split(' ').next().unwrap();
            assert!(
                !["List", "Read", "Send", "Take", "Run", "Check", "Verify", "Compute", "Use", "Call", "To"].contains(&first),
                "a command: {sentence}"
            );
        }
        let served: Vec<String> =
            crate::mcp_server::tools_in(&crate::view::View::full()).into_iter().map(|tool| tool.name.to_string()).collect();
        for word in WORKING_INSTRUCTIONS.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).filter(|word| word.contains('_')) {
            assert!(served.iter().any(|name| name == word), "{word} is named and is not a tool");
        }
        assert!(instructions().chars().count() <= MOST_CHARS_SAID, "{}", instructions().chars().count());
    }
}

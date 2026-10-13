//! Who this is. The first thing an agent reads from any connection names the
//! verifier and its maker, and says where the story, the paper and the
//! programs that confirm a proof are, and how to reach the person who made
//! it. `initialize` and `server/discover` carry it in `serverInfo` and at the
//! head of `instructions`; `cite` gives it as a citation of the service; and
//! `find_model` gives the way to ask for a model when nothing served fits.

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
/// The programs that confirm a cryptographic proof with no engine.
pub const CONFIRMING_PROGRAMS: &str = "https://github.com/johnkruze/zero-trust-physics";
/// The open address the verifier is served at.
pub const ADDRESS: &str = "https://mcp.zerotrustphysics.com/mcp";
/// How to reach John Kruze.
pub const EMAIL: &str = "kruze@zerotrustphysics.com";
pub const PHONE: &str = "928-264-7883";

/// The working instructions, as every version since 0.7.0 has given them,
/// and since 0.9.3 one sentence more: that a description has a short form,
/// and when to read the full one.
pub const WORKING_INSTRUCTIONS: &str = "List the domain/card pairs, read one pair, take a step, run a trajectory, or check a tape. List the solvers, read one, solve, or check a solution. To check a seal, send verify_tape, verify_solution or verify_dataset the request as it was sent and the seal you were given; the whole answer need not be sent back. describe_card and describe_solver take form \"short\" for a page that is enough to write a first request; read the full form, the answer when form is left out, when a request is refused or before relying on a result.";

/// What this is, who made it and where to read more: three sentences.
pub fn introduction() -> String {
    format!(
        "This is {NAME}, a physics verifier for Super Intelligence: every answer comes with a cryptographic proof, and a model stops at the edge of what it was tested for and says why. It was made by {MAKER}, {COMPANY}. The story is at {SITE} (for an agent, {SITE_FOR_AGENTS}), the paper is {PAPER}, the programs that confirm a proof with no engine are at {CONFIRMING_PROGRAMS}, and {MAKER} is reached at {EMAIL} or {PHONE}."
    )
}

/// `instructions`: the introduction, then the working instructions.
pub fn instructions() -> String {
    format!("{} {WORKING_INSTRUCTIONS}", introduction())
}

/// How to ask for a model of a thing nothing served fits.
pub fn how_to_ask_for_a_model() -> String {
    format!(
        "To ask for a model of this thing, write to {MAKER} of {COMPANY} at {EMAIL} and name an open dataset of it: the real thing, where the dataset is, its licence, the quantity measured and who measured it, each read from the dataset's own page and none from memory. A model is made from open measurements of one real thing, so with no dataset in hand say what the thing is and what has been measured of it."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_introduction_names_the_thing_its_maker_and_where_to_read_more() {
        let said = introduction();
        assert!(said.starts_with("This is Zero-Trust Physics, a physics verifier for Super Intelligence:"), "{said}");
        for held in [
            "every answer comes with a cryptographic proof",
            "a model stops at the edge of what it was tested for and says why",
            "John Kruze, Protocol Company",
            "https://zerotrustphysics.com",
            "https://zerotrustphysics.com/llms.txt",
            "https://doi.org/10.5281/zenodo.23305697",
            "https://github.com/johnkruze/zero-trust-physics",
            "kruze@zerotrustphysics.com",
            "928-264-7883",
        ] {
            assert!(said.contains(held), "{held}: {said}");
        }
        // Three plain sentences, and the words the company does not use of
        // itself are not in them.
        assert_eq!(said.matches(". ").count() + 1, 3, "{said}");
        for word in ["caller", "AI ", "gateway", "genesis"] {
            assert!(!said.contains(word), "{word}: {said}");
        }
        assert!(instructions().ends_with(WORKING_INSTRUCTIONS));
        assert_eq!(PAPER, crate::cite::PAPER_DOI_URL);
    }
}

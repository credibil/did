//! # DID Web Resolver
//!
//! The `did:web` method uses a web domain's reputation to confer trust. This
//! module provides a resolver for `did:web` DIDs.
//!
//! See:
//!
//! - <https://w3c-ccg.github.io/did-method-web>
//! - <https://w3c.github.io/did-resolution>

use std::sync::LazyLock;

use regex::Regex;
use serde_json::json;

use super::DidWeb;
use crate::DidResolver;
use crate::error::Error;
use crate::resolve::{ContentType, Metadata, Options, Resolved};

static DID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^did:web:(?<identifier>[a-zA-Z0-9.\\-:\\%]+)$").expect("should compile")
});

impl DidWeb {
    /// Resolve a `did:web` DID URL to a DID document.
    ///
    /// # Errors
    ///
    /// Will fail if the DID URL is invalid or the DID document cannot be
    /// found.
    pub async fn resolve(
        did: &str, _: Option<Options>, resolver: impl DidResolver,
    ) -> crate::Result<Resolved> {
        // Steps 1-5. Generate the URL to fetch the DID document.
        let url = Self::url(did)?;

        // 6. Perform an HTTP GET request to the URL using an agent that can
        //    successfully negotiate a secure HTTPS connection, which enforces the
        //    security requirements as described in 2.6 SecOps and privacy
        //    considerations.
        let document = resolver.resolve(&url).await.map_err(Error::Other)?;

        // 7. When performing the DNS resolution during the HTTP GET request, the client
        //    SHOULD utilize [RFC8484] in order to prevent tracking of the identity
        //    being resolved.

        Ok(Resolved {
            context: "https://w3id.org/did-resolution/v1".into(),
            metadata: Metadata {
                content_type: ContentType::DidLdJson,
                additional: Some(json!({
                    "pattern": "^did:web:(?<identifier>[a-zA-Z0-9.\\-:\\%]+)$",
                    "did": {
                        "didString": did,
                        "methodSpecificId": did[8..],
                        "method": "web"
                    }
                })),
                ..Metadata::default()
            },
            document: Some(document),
            ..Resolved::default()
        })
    }

    /// Convert a `did:web` URL to an HTTP URL pointing to the location of the
    /// DID document.
    ///
    /// # Errors
    ///
    /// Will fail if the DID URL is not a valid `did:web` URL.
    pub fn url(did: &str) -> crate::Result<String> {
        let Some(caps) = DID_REGEX.captures(did) else {
            return Err(Error::InvalidDid("DID is not a valid did:web".to_string()));
        };
        let identifier = &caps["identifier"];

        // 1. Replace ":" with "/" in the method specific identifier to obtain the fully
        //    qualified domain name and optional path.
        let domain = identifier.replace(':', "/");

        // 2. If the domain contains a port percent decode the colon.
        let domain = domain.replace("%3A", ":");

        // 3. Generate an HTTPS URL to the expected location of the DID document by
        //    prepending https://.
        let mut url = format!("https://{domain}");

        // 4. If no path has been specified in the URL, append /.well-known.
        if !identifier.contains(':') {
            url = format!("{url}/.well-known");
        }

        // 5. Append /did.json to complete the URL.
        url = format!("{url}/did.json");

        Ok(url)
    }
}

#[cfg(test)]
mod test {
    use anyhow::anyhow;
    use insta::assert_json_snapshot as assert_snapshot;

    use super::*;
    use crate::document::Document;

    #[derive(Clone)]
    struct MockResolver;
    impl DidResolver for MockResolver {
        async fn resolve(&self, _url: &str) -> anyhow::Result<Document> {
            serde_json::from_slice(include_bytes!("did-ecdsa.json"))
                .map_err(|e| anyhow!("issue deserializing document: {e}"))
        }
    }

    #[tokio::test]
    async fn resolve_normal() {
        const DID_URL: &str = "did:web:demo.credibil.io";

        let resolved = DidWeb::resolve(DID_URL, None, MockResolver).await.expect("should resolve");
        assert_snapshot!("document", resolved.document);
        assert_snapshot!("metadata", resolved.metadata);
    }

    #[test]
    fn should_construct_url() {
        let did = "did:web:domain.with-hypens.computer";
        let url = DidWeb::url(did).expect("should construct URL");
        assert_eq!(url, "https://domain.with-hypens.computer/.well-known/did.json");
    }
}

//! # DID Web with Verifiable History Resolver
//!
//! Resolution of a DID for the `did:webvh` method.
//!
//! See: <https://identity.foundation/didwebvh/next/>

use std::sync::LazyLock;

use regex::Regex;
use serde_json::json;

use super::DidWebVh;
use crate::{
    ContentType, DidResolver, Error, Metadata,
    resolution::{Options, Resolved},
};

static DID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("^did:webvh:(?<identifier>[a-zA-Z0-9.\\-:\\%]+)$").expect("should compile")
});

impl DidWebVh {
    /// Resolve a `did:webvh` DID URL to a DID document.
    ///
    /// The first step of the resolution is to retrieve and parse the DID list
    /// document. See further functions in this implementation to help with
    /// resolution steps.
    ///
    /// # Errors
    ///
    /// Will fail if the DID URL is invalid or the DID list document cannot be
    /// found.
    pub async fn resolve(
        did: &str, options: Option<Options>, resolver: impl DidResolver,
    ) -> crate::Result<Resolved> {
        // Steps 1-7. Generate the URL to fetch the DID list document.
        let url = Self::url(did)?;

        // 8. The content type for the did.jsonl file SHOULD be text/jsonl.
        if let Some(opts) = options {
            if let Some(content_type) = opts.accept {
                if content_type != ContentType::JsonL {
                    return Err(Error::RepresentationNotSupported(
                        "Content type must be text/json".to_string(),
                    ));
                }
            }
        }

        // Perform an HTTP GET request to the URL using an agent that can
        // successfully negotiate a secure HTTPS connection.
        // The URL
        let document = resolver.resolve(&url).await.map_err(Error::Other)?;

        Ok(Resolved {
            context: "https://w3id.org/did-resolution/v1".into(),
            metadata: Metadata {
                content_type: ContentType::DidLdJson,
                additional: Some(json!({
                    "pattern": "^did:webvh:(?<identifier>[a-zA-Z0-9.\\-:\\%]+)$",
                    "did": {
                        "didString": did,
                        "methodSpecificId": did[8..],
                        "method": "webvh"
                    }
                })),
                ..Metadata::default()
            },
            document: Some(document),
            ..Resolved::default()
        })
    }

    /// Convert a `did:webvh` URL to an HTTP URL pointing to the location of the
    /// DID list document.
    ///
    /// # Errors
    ///
    /// Will fail if the DID URL is invalid.
    ///
    /// TODO: Extend for witnesses URL.
    /// TODO: Extend for resolving a DID path (such as <did>/whois or
    /// <did>/path/to/file).
    ///
    /// <https://identity.foundation/didwebvh/#the-did-to-https-transformation>
    ///
    pub fn url(did: &str) -> crate::Result<String> {
        let Some(caps) = DID_REGEX.captures(did) else {
            return Err(Error::InvalidDid("DID is not a valid did:webvh".to_string()));
        };
        // 1. Remove the literal `did:webvh:` prefix from the DID URL.
        let scid_and_identifier = &caps["identifier"];

        // 2. Remove the `SCID` by removing the text up to and including the
        // first `:` character.
        let Some(identifier) = scid_and_identifier.split_once(':').map(|x| x.1) else {
            return Err(Error::InvalidDid("DID is not a valid did:webvh - no SCID".to_string()));
        };

        // 3. Replace `:` with `/` in the method-specific identifier to obtain
        // the fully qualified domain name and optional path.
        let mut domain = identifier.replace(':', "/");

        // 4. If there is no optional path, append `/.well-known` to the URL.
        if !identifier.contains(':') {
            domain.push_str("/.well-known");
        }

        // 5. If the domain contains a port, percent-decode the colon.
        let domain = domain.replace("%3A", ":");

        // 6. Prepend `https://` to the domain to generate the URL.
        let url = format!("https://{domain}");

        // 7. Append `/did.jsonl` to the URL to complete it.
        // TODO: witness and path extensions to be catered for here.
        let url = format!("{url}/did.jsonl");

        Ok(url)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_construct_default_url() {
        let did = "did:webvh:z6Mk3vz:domain.with-hyphens.computer";
        let url = DidWebVh::url(did).unwrap();
        assert_eq!(url, "https://domain.with-hyphens.computer/.well-known/did.jsonl");
    }

    #[test]
    fn should_construct_path_url() {
        let did = "did:webvh:z6Mk3vz:domain.with-hyphens.computer:dids:issuer";
        let url = DidWebVh::url(did).unwrap();
        assert_eq!(url, "https://domain.with-hyphens.computer/dids/issuer/did.jsonl");
    }

    #[test]
    fn should_construct_port_url() {
        let did = "did:webvh:z6Mk3vz:domain.with-hyphens.computer%3A8080";
        let url = DidWebVh::url(did).unwrap();
        assert_eq!(url, "https://domain.with-hyphens.computer:8080/.well-known/did.jsonl");
    }
}

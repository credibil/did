//! Tests for the creation of a new `did:webvh` document and associated
//! log entry.

use credibil_did::{
    KeyPurpose,
    core::{Kind, OneMany},
    document::{MethodType, Service, VerificationMethod},
    operation::document::{Create, DocumentBuilder, VerificationMethodBuilder, VmKeyId},
    webvh::{Witness, WitnessWeight, create::CreateBuilder, url::default_did},
};
use kms::new_keyring;
use serde_json::Value;

use credibil_did::webvh::SCID_PLACEHOLDER;

// Test the happy path of creating a new `did:webvh` document and associated log
// entry. Should just work without errors.
#[tokio::test]
async fn create_success() {
    let domain_and_path = "https://credibil.io/issuers/example";

    let signer = new_keyring();
    let auth_jwk = signer.verifying_key_jwk().await.expect("should get JWK key");
    let update_multi = signer.verifying_key_multibase().await.expect("should get multibase key");
    let update_keys = vec![update_multi.clone()];
    let update_keys: Vec<&str> = update_keys.iter().map(|s| s.as_str()).collect();

    let did = default_did(domain_and_path).expect("should get default DID");

    let vm_jwk = signer.verifying_key_jwk().await.expect("should get JWK key");
    let vm = VerificationMethodBuilder::new(&vm_jwk)
        .key_id(&did, VmKeyId::Authorization(auth_jwk))
        .expect("should apply key ID")
        .method_type(&MethodType::Ed25519VerificationKey2020)
        .expect("should apply method type")
        .build();
    let vm_kind = Kind::<VerificationMethod>::Object(vm.clone());
    let service = Service {
        id: format!("did:webvh:{}:example.com#whois", SCID_PLACEHOLDER),
        type_: "LinkedVerifiablePresentation".to_string(),
        service_endpoint: OneMany::<Kind<Value>>::One(Kind::String(
            "https://example.com/.well-known/whois".to_string(),
        )),
    };
    let doc = DocumentBuilder::<Create>::new(&did)
        .add_verification_method(&vm_kind, &KeyPurpose::VerificationMethod)
        .expect("should apply verification method")
        .add_service(&service)
        .build();

    let next_multi =
        new_keyring().verifying_key_multibase().await.expect("should get multibase key");

    let witnesses = Witness {
        threshold: 60,
        witnesses: vec![
            WitnessWeight {
                id: new_keyring().did().to_string(),
                weight: 50,
            },
            WitnessWeight {
                id: new_keyring().did().to_string(),
                weight: 40,
            },
        ],
    };

    let result = CreateBuilder::new(&update_keys, &doc)
        .expect("should create builder")
        .next_key(&next_multi)
        .portable(false)
        .witness(&witnesses)
        .expect("witness information should be applied")
        .ttl(60)
        .build(&signer)
        .await
        .expect("should build document");

    let log_entry = serde_json::to_string(&result.log[0]).expect("should serialize log entry");
    println!("{log_entry}");
}

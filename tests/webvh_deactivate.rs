//! Tests for the deactivation of a `did:webvh` document and associated log
//! entries.
//!

use credibil_did::KeyPurpose;
use credibil_did::core::{Kind, OneMany};
use credibil_did::document::{
    DocumentBuilder, MethodType, Service, VerificationMethod, VerificationMethodBuilder, VmKeyId,
};
use credibil_did::webvh::{
    CreateBuilder, DeactivateBuilder, SCID_PLACEHOLDER, UpdateBuilder, Witness, WitnessWeight,
    default_did,
};
use credibil_infosec::Signer;
use kms::Keyring;
use serde_json::Value;

// Test the happy path of creating then deactivating a `did:webvh` document and
// log entries. Should just work without errors.
#[tokio::test]
async fn create_then_deactivate() {
    let domain_and_path = "https://credibil.io/issuers/example";

    let mut signer = Keyring::new();
    let update_jwk = signer.jwk("signing").expect("should get signing key");
    let update_multi = signer.multibase("signing").expect("should get multibase key");
    let update_keys = vec![update_multi.clone()];
    let update_keys: Vec<&str> = update_keys.iter().map(|s| s.as_str()).collect();

    let id_jwk = signer.jwk("id").expect("should get key");

    let did = default_did(domain_and_path).expect("should get default DID");

    let vm = VerificationMethodBuilder::new(&update_jwk)
        .key_id(&did, VmKeyId::Authorization(id_jwk))
        .expect("should apply key ID")
        .method_type(&MethodType::Ed25519VerificationKey2020)
        .expect("should apply method type")
        .build();
    let vm_kind = Kind::<VerificationMethod>::Object(vm.clone());
    signer.set_verification_method("signing").expect("should set verification method");
    let service = Service {
        id: format!("did:webvh:{}:example.com#whois", SCID_PLACEHOLDER),
        type_: "LinkedVerifiablePresentation".to_string(),
        service_endpoint: OneMany::<Kind<Value>>::One(Kind::String(
            "https://example.com/.well-known/whois".to_string(),
        )),
    };
    let doc = DocumentBuilder::new(&did)
        .add_verification_method(&vm_kind, &KeyPurpose::VerificationMethod)
        .expect("should apply verification method")
        .add_service(&service)
        .build();

    let next_multi = signer.next_multibase("signing").expect("should get next key");

    let mut witness_keyring1 = Keyring::new();
    witness_keyring1.set_verification_method("signing").expect("should set verification method");
    let mut witness_keyring2 = Keyring::new();
    witness_keyring2.set_verification_method("signing").expect("should set verification method");
    let witnesses = Witness {
        threshold: 60,
        witnesses: vec![
            WitnessWeight {
                id: witness_keyring1
                    .verification_method()
                    .await
                    .expect("should get verifying key as did:key"),
                weight: 50,
            },
            WitnessWeight {
                id: witness_keyring2
                    .verification_method()
                    .await
                    .expect("should get verifying key as did:key"),
                weight: 40,
            },
        ],
    };

    let create_result = CreateBuilder::new()
        .document(&doc)
        .expect("should apply document")
        .update_keys(&update_keys)
        .expect("should apply update keys")
        .next_key(&next_multi)
        .portable(false)
        .witness(&witnesses)
        .expect("witness information should be applied")
        .ttl(60)
        .signer(&signer)
        .build()
        .await
        .expect("should build document");

    let deactivate_result = DeactivateBuilder::new(&create_result.log)
        .expect("should create builder")
        .build(&signer)
        .await
        .expect("should build deactivated document");

    let logs = serde_json::to_string(&deactivate_result.log).expect("should serialize log entries");
    println!("{logs}");

    // Should have 3 log entries: create, nullify next keys, deactivate.
    assert_eq!(deactivate_result.log.len(), 3);
}

// Test the happy path of updating then deactivating a `did:webvh` document and
// log entries. Should just work without errors.
#[tokio::test]
async fn update_then_deactivate() {
    // --- Create --------------------------------------------------------------

    let domain_and_path = "https://credibil.io/issuers/example";

    let mut signer = Keyring::new();
    let update_jwk = signer.jwk("signing").expect("should get signing key");
    let update_multi = signer.multibase("signing").expect("should get multibase key");
    let update_keys = vec![update_multi.clone()];
    let update_keys: Vec<&str> = update_keys.iter().map(|s| s.as_str()).collect();

    let id_jwk = signer.jwk("id").expect("should get key");

    let did = default_did(domain_and_path).expect("should get default DID");

    let vm = VerificationMethodBuilder::new(&update_jwk)
        .key_id(&did, VmKeyId::Authorization(id_jwk))
        .expect("should apply key ID")
        .method_type(&MethodType::Ed25519VerificationKey2020)
        .expect("should apply method type")
        .build();
    let vm_kind = Kind::<VerificationMethod>::Object(vm.clone());
    signer.set_verification_method("signing").expect("should set verification method");
    let service = Service {
        id: format!("did:webvh:{}:example.com#whois", SCID_PLACEHOLDER),
        type_: "LinkedVerifiablePresentation".to_string(),
        service_endpoint: OneMany::<Kind<Value>>::One(Kind::String(
            "https://example.com/.well-known/whois".to_string(),
        )),
    };
    let doc = DocumentBuilder::new(&did)
        .add_verification_method(&vm_kind, &KeyPurpose::VerificationMethod)
        .expect("should apply verification method")
        .add_service(&service)
        .build();

    let next_multi = signer.next_multibase("signing").expect("should get next key");

    let mut witness_keyring1 = Keyring::new();
    witness_keyring1.set_verification_method("signing").expect("should set verification method");
    let mut witness_keyring2 = Keyring::new();
    witness_keyring2.set_verification_method("signing").expect("should set verification method");
    let witnesses = Witness {
        threshold: 60,
        witnesses: vec![
            WitnessWeight {
                id: witness_keyring1
                    .verification_method()
                    .await
                    .expect("should get verifying key as did:key"),
                weight: 50,
            },
            WitnessWeight {
                id: witness_keyring2
                    .verification_method()
                    .await
                    .expect("should get verifying key as did:key"),
                weight: 40,
            },
        ],
    };

    let create_result = CreateBuilder::new()
        .document(&doc)
        .expect("should apply document")
        .update_keys(&update_keys)
        .expect("should apply update keys")
        .next_key(&next_multi)
        .portable(false)
        .witness(&witnesses)
        .expect("witness information should be applied")
        .ttl(60)
        .signer(&signer)
        .build()
        .await
        .expect("should build document");

    // --- Update --------------------------------------------------------------

    let doc = create_result.document.clone();

    // Rotate the signing key.
    signer.rotate().expect("should rotate keys on signer");
    let new_update_jwk = signer.jwk("signing").expect("should get signing key");
    let new_update_multi = signer.multibase("signing").expect("should get multibase key");
    let new_update_keys = vec![new_update_multi.clone()];
    let new_update_keys: Vec<&str> = new_update_keys.iter().map(|s| s.as_str()).collect();

    let new_next_multi = signer.next_multibase("signing").expect("should get next key");
    let new_next_keys = vec![new_next_multi.clone()];
    let new_next_keys: Vec<&str> = new_next_keys.iter().map(|s| s.as_str()).collect();
    let id_jwk = signer.jwk("id").expect("should get key");

    let vm = VerificationMethodBuilder::new(&new_update_jwk)
        .key_id(&did, VmKeyId::Authorization(id_jwk))
        .expect("should apply key ID")
        .method_type(&MethodType::Ed25519VerificationKey2020)
        .expect("should apply method type")
        .build();
    let vm_kind = Kind::<VerificationMethod>::Object(vm.clone());
    signer.set_verification_method("signing").expect("should set verification method");

    // Add another reference-based verification method as a for-instance.
    let vm_list = doc.verification_method.clone().expect("should get verification methods");
    let vm = vm_list.first().expect("should get first verification method");
    let auth_vm = Kind::<VerificationMethod>::String(vm.id.clone());

    // Construct a new document from the existing one.
    let doc = DocumentBuilder::from(&doc)
        .add_verification_method(&vm_kind, &KeyPurpose::VerificationMethod)
        .expect("should apply verification method")
        .add_verification_method(&auth_vm, &KeyPurpose::Authentication)
        .expect("should add verification method")
        .build();

    // Create an update log entry and skip witness verification of existing log.
    let update_result = UpdateBuilder::new(create_result.log.as_slice(), None, &doc)
        .await
        .expect("should create builder")
        .rotate_keys(&new_update_keys, &new_next_keys)
        .expect("should rotate keys on builder")
        .signer(&signer)
        .build()
        .await
        .expect("should build document");

    // --- Deactivate ----------------------------------------------------------

    signer.rotate().expect("should rotate keys on signer");

    let new_update_multi = signer.multibase("signing").expect("should get multibase key");
    let new_update_keys = vec![new_update_multi.clone()];
    let new_update_keys: Vec<&str> = new_update_keys.iter().map(|s| s.as_str()).collect();

    let new_next_multi = signer.next_multibase("signing").expect("should get next key");
    let new_next_keys = vec![new_next_multi.clone()];
    let new_next_keys: Vec<&str> = new_next_keys.iter().map(|s| s.as_str()).collect();

    let deactivate_result = DeactivateBuilder::new(&update_result.log)
        .expect("should create builder")
        .rotate_keys(&new_update_keys, &new_next_keys)
        .expect("should rotate keys on builder")
        .build(&signer)
        .await
        .expect("should build deactivated document");

    let logs = serde_json::to_string(&deactivate_result.log).expect("should serialize log entries");
    println!("{logs}");

    // Should have 4 log entries: create, update, nullify next keys, deactivate.
    assert_eq!(deactivate_result.log.len(), 4);
}

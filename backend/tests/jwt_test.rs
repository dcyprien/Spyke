use backend::application::utils::jwt::{generate_token, verify_token};
use backend::application::dto::apperror::AppError;
use uuid::Uuid;
use std::env;
use std::sync::Mutex;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Header, EncodingKey};
use serde_json::json;

// Mutex pour l'environnement (Global Lock)
// Empêche deux tests de modifier JWT_SECRET en même temps
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn setup_env() {
    env::set_var("JWT_SECRET", "super_secret_test_key_must_be_long_enough");
}

fn teardown_env() {
    env::remove_var("JWT_SECRET");
}

#[test]
fn test_generate_token_success() {
    let _guard = ENV_LOCK.lock().unwrap();
    setup_env();
    
    let user_id = Uuid::new_v4();
    let username = "test_user".to_string();

    let result = generate_token(user_id, username);
    
    assert!(result.is_ok());
    let token = result.unwrap();
    
    // Structure basique d'un JWT : Header.Payload.Signature
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
}

#[test]
fn test_generate_token_missing_secret() {
    let _guard = ENV_LOCK.lock().unwrap();
    teardown_env(); // On s'assure qu'il n'y a pas de variables

    let user_id = Uuid::new_v4();
    let username = "fail_user".to_string();

    let result = generate_token(user_id, username);
    
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(), 
        AppError::InternalServerError("JWT_SECRET not found in environment".to_string())
    );
}

#[test]
fn test_verify_token_valid() {
    let _guard = ENV_LOCK.lock().unwrap();
    setup_env();
    
    let user_id = Uuid::new_v4();
    let username = "valid_user".to_string();
    
    // 1. Génération
    let token = generate_token(user_id, username.clone()).unwrap();
    
    // 2. Vérification
    let result = verify_token(&token);
    assert!(result.is_ok());
    
    let claims = result.unwrap();
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.username, username);
    
    // Vérifier que l'expiration est dans le futur
    let now = Utc::now().timestamp() as usize;
    assert!(claims.exp > now);
}

#[test]
fn test_verify_token_expired() {
    let _guard = ENV_LOCK.lock().unwrap();
    setup_env();

    // Création manuelle d'un token expiré
    let claims = json!({
        "sub": Uuid::new_v4(),
        "username": "expired_user",
        // Expire il y a 1 heure
        "exp": (Utc::now() - Duration::hours(1)).timestamp(), 
        "iat": (Utc::now() - Duration::hours(2)).timestamp(),
    });
    
    let key = EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_ref());
    let token = encode(&Header::default(), &claims, &key).unwrap();

    let result = verify_token(&token);
    
    assert!(result.is_err());
    
    if let Err(AppError::InternalServerError(msg)) = result {
        // jsonwebtoken renvoie "ExpiredSignature"
        assert!(msg.contains("ExpiredSignature"), "Message attendu 'ExpiredSignature', reçu: '{}'", msg);
    } else {
        panic!("Mauvais type d'erreur retourné");
    }
}

#[test]
fn test_verify_token_wrong_signature() {
    let _guard = ENV_LOCK.lock().unwrap();
    setup_env();

    // 1. Générer un token valide avec une "Fausse Clé"
    let claims = json!({
        "sub": Uuid::new_v4(),
        "username": "hacker",
        "exp": (Utc::now() + Duration::hours(1)).timestamp(),
        "iat": Utc::now().timestamp(),
    });
    
    let fake_key = EncodingKey::from_secret("WRONG_KEY".as_ref());
    let token = encode(&Header::default(), &claims, &fake_key).unwrap();

    // 2. Essayer de vérifier avec la "Vraie Clé" (définie dans setup_env)
    let result = verify_token(&token);
    
    assert!(result.is_err());
    
    if let Err(AppError::InternalServerError(msg)) = result {
        // jsonwebtoken renvoie "InvalidSignature"
        assert!(msg.contains("InvalidSignature"), "Message attendu 'InvalidSignature', reçu: '{}'", msg);
    } else {
        panic!("Mauvais type d'erreur retourné");
    }
}

#[test]
fn test_verify_token_malformed() {
    let _guard = ENV_LOCK.lock().unwrap();
    setup_env();

    let malformed_token = "ceci.nest.pas.un.token";
    let result = verify_token(malformed_token);
    
    assert!(result.is_err());
    // L'erreur peut varier selon le parser (souvent base64 error ou invalid format)
}

#[test]
fn test_verify_token_tampered_payload() {
    let _guard = ENV_LOCK.lock().unwrap();
    setup_env();

    let user_id = Uuid::new_v4();
    let token = generate_token(user_id, "user".to_string()).unwrap();

    // On coupe le token en 3 parties
    let parts: Vec<&str> = token.split('.').collect();
    
    // On garde Header et Signature, mais on remplace le Payload par du "bruit"
    // Note: Modifier le payload invalide la signature mathématiquement
    let tampered_payload = "ew0KICAic3ViIjogIjEyMzQ1Njc4OTAiLA0KICAibmFtZSI6ICJKb2huIERvZSIsDQogICJpYXQiOiAxNTE2MjM5MDIyDQp9"; // Un payload base64 valide quelconque
    let tampered_token = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);

    let result = verify_token(&tampered_token);
    
    assert!(result.is_err());
    if let Err(AppError::InternalServerError(msg)) = result {
        assert!(msg.contains("InvalidSignature"), "La signature ne devrait plus correspondre au payload modifié");
    }
}

#[test]
fn test_verify_missing_secret() {
    let _guard = ENV_LOCK.lock().unwrap();
    teardown_env(); // Pas de variable d'env

    let token = "some.valid.structure"; // Le contenu importe peu, ça doit fail avant
    let result = verify_token(token);

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        AppError::InternalServerError("JWT_SECRET not found in environment".to_string())
    );
}
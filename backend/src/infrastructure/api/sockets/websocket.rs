use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use crate::AppState;
use crate::domain::models::{server_member, user};
use crate::domain::models::user::UserStatus;
use futures::{sink::SinkExt, stream::StreamExt};
use crate::application::utils::jwt::verify_token;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast;

#[derive(Deserialize)]
struct AuthMessage {
    r#type: String,
    token: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut authenticated_user_id = None;
    let mut current_username = String::new();

    // --- PHASE 1 : AUTH ---
    println!("WS: Attente authentification...");
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(payload) = serde_json::from_str::<AuthMessage>(&text) {
                if payload.r#type == "auth" {
                    match verify_token(&payload.token) {
                        Ok(claims) => {
                            authenticated_user_id = Some(claims.sub);
                            current_username = claims.username;
                            
                            let success_msg = json!({
                                "type": "auth_success",
                                "data": { "user_id": claims.sub }
                            });
                            sender.send(Message::Text(success_msg.to_string().into())).await.ok();
                            
                            break; // Sort de la boucle d'auth
                        },
                        Err(_) => {
                            let _ = sender.send(Message::Text(json!({"error": "Invalid token"}).to_string().into())).await;
                            return;
                        }
                    }
                }
            }
        }
    }

    let user_id = match authenticated_user_id {
        Some(id) => id,
        None => {
            println!("WS: Connection fermée avant auth");
            return;
        }
    };

    // --- MISE A JOUR STATUS : ONLINE ---
    let db = &state.db;
    let user_update = user::ActiveModel {
        id: Set(user_id),
        status: Set(UserStatus::Online),
        ..Default::default()
    };
    if let Err(e) = user_update.update(db.as_ref()).await {
        println!("WS: Erreur update status online: {}", e);
    }

    // Broadcast Online pour tous les serveurs de l'utilisateur
    {
        let memberships = server_member::Entity::find()
            .filter(server_member::Column::UserId.eq(user_id))
            .all(state.db.as_ref())
            .await;
        if let Ok(members) = memberships {
            for m in members {
                let msg = serde_json::json!({
                    "type": "user_status_change",
                    "data": {
                        "server_id": m.server_id,
                        "user_id": user_id,
                        "status": "online"
                    }
                });
                let _ = state.tx.send(msg.to_string());
            }
        }
    }

    // --- PHASE 2 : LOGIQUE METIER ---

    // 1. Chargement initial des serveurs
    let user_server_ids_result = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(user_id))
        .all(state.db.as_ref())
        .await;

    let user_server_ids: Vec<i32> = match user_server_ids_result {
        Ok(members) => {
            let ids: Vec<i32> = members.into_iter().map(|m| m.server_id).collect();
            // println!("WS: User {} est membre des serveurs {:?}", user_id, ids); // Commenté pour réduire logs
            ids
        },
        Err(e) => { println!("WS: Erreur DB: {}", e); return; }
    };

    let tx = state.tx.clone();
    let username_for_recv = current_username.clone();
    let mut rx = state.tx.subscribe();

    // Tâche d'envoi (Broadcast -> WebSocket)
    let mut send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg_string) => {
                   if let Ok(json_msg) = serde_json::from_str::<Value>(&msg_string) {
                        // Extraction robuste de l'ID du serveur
                        let maybe_server_id = json_msg.get("data")
                            .and_then(|data| data.get("server_id"))
                            .and_then(|v| {
                                v.as_i64().map(|n| n as i32)
                                .or_else(|| v.as_str().and_then(|s| s.parse::<i32>().ok()))
                            });

                        if let Some(msg_server_id) = maybe_server_id {
                             if user_server_ids.contains(&msg_server_id) {
                                if let Err(e) = sender.send(Message::Text(msg_string.into())).await {
                                    // println!("WS: Erreur d'envoi socket: {}", e);
                                    break; 
                                }
                            }
                        }
                    }
                },
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Tâche de réception
    let mut recv_task = tokio::spawn(async move {
         while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                 if let Ok(client_msg) = serde_json::from_str::<Value>(&text) {
                     let msg_type = client_msg["type"].as_str().unwrap_or("");
                     if msg_type == "typing_start" || msg_type == "typing_stop" {
                         let broadcast_msg = json!({
                            "type": msg_type,
                            "data": {
                                "server_id": client_msg["server_id"],
                                "channel_id": client_msg["channel_id"],
                                "user_id": authenticated_user_id,
                                "username": username_for_recv 
                            }
                        });
                        let _ = tx.send(broadcast_msg.to_string());
                     }
                 }
            }
         }
    });

    tokio::select! {
        _ = (&mut send_task) => {},
        _ = (&mut recv_task) => {},
    };
    
    send_task.abort();
    recv_task.abort();

    // --- MISE A JOUR STATUS : OFFLINE (Disconnect) ---
    // Update DB
    let user_update_offline = user::ActiveModel {
        id: Set(user_id),
        status: Set(UserStatus::Offline),
        ..Default::default()
    };
    // Note: state.db est cloné implicitement via AppState qui est un Arc en général, ou on le clone ici
    // Ici on a besoin de recréer une connexion user_server_ids scope, mais 'state' a été déplacé ? 
    // Non, 'state' est passé par valeur, mais AppState contient Arc, donc c'est bon.
    if let Err(e) = user_update_offline.update(state.db.as_ref()).await {
        println!("WS: Erreur update status offline: {}", e);
    }

    // Broadcast Disconnect (on réutilise la liste user_server_ids calculée au début)
    // Attention : tx et user_server_ids ont été moved dans les tasks ?
    // Non, user_server_ids a été cloné ou déplacé dans send_task ? 
    // Au-dessus : `if user_server_ids.contains...` -> cela déplace user_server_ids dans la closure async move.
    
    // CORRECTION : Il faut recalculer ou cloner ids avant le spawn des tâches.
    // Pour simplifier, on refait un fetch rapide des serveurs ou on clone avant.
    // Comme user_server_ids a été move, on ne peut plus l'utiliser ici.
    // Solution : On refetch rapidement pour être sûr (au cas où il a quitté un serveur entre temps).
    
    if let Ok(members) = server_member::Entity::find()
        .filter(server_member::Column::UserId.eq(user_id))
        .all(state.db.as_ref())
        .await 
    {
         for member in members {
            let msg = json!({
                "type": "user_status_change",
                "data": {
                    "server_id": member.server_id,
                    "user_id": user_id,
                    "status": "offline"
                }
            });
            let _ = state.tx.send(msg.to_string());
        }
    }

    println!("WS: Session terminée pour {}", user_id);
}
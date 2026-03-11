"use client";

import React, { useRef, useState, useEffect } from "react";
import { useAuth } from "../app/context";
import { useLang } from "../app/langContext";

export interface Channel {
  id: string;
  server_id: number;
  name: string;
}

export interface Server {
  id: number;
  name: string;
  description: string;
  invitcode: number;
  owner_id: string;
  admins?: string[]; // AJOUT DU CHAMP OPTIONNEL (le temps que le backend compile)
  channels?: Channel[];
  members?: string[];
}

type ServerBarProps = {
  onServerSelect?: (s: Server | null) => void;
  onChannelSelect?: (c: Channel | null) => void;
  onJoinServer?: () => void;
  onCreateServer?: () => void;
  mobileTab?: string;
};

export default function ServerBar({ onServerSelect, onChannelSelect, mobileTab }: ServerBarProps) {
  const { servers, addServer, setServers, refreshUserData, user, socket } = useAuth();
  const { t } = useLang();
  
  // NOUVEAU: État pour gérer l'onglet actif
  const [activeTab, setActiveTab] = useState<"servers" | "dms">("servers");
  // NOUVEAU: État temporaire pour la liste des DMs (à relier au backend ensuite)
  const [dms, setDms] = useState<any[]>([]);

  const [expandedServerId, setExpandedServerId] = useState<number | null>(null);
  const [selectedServerId, setSelectedServerId] = useState<number | null>(null);
  const [selectedChannelId, setSelectedChannelId] = useState<string | null>(null);
  
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [serverName, setServerName] = useState("");
  const [serverDescription, setServerDescription] = useState("");
  
  const [showJoinInput, setShowJoinInput] = useState(false);
  const [joinCode, setJoinCode] = useState("");
  const [joinServerId, setJoinServerId] = useState(""); // Nouvel état pour l'ID

  // --- ACTIONS SERVEUR ---

  const handleCreateServer = async () => {
    try {
      const token = localStorage.getItem("access_token");
      const res = await fetch("http://localhost:3000/servers", {
        method: "POST",
        headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
        body: JSON.stringify({ name: serverName, description: serverDescription }),
      });
      if (res.ok) {
        const newServer = await res.json();
        addServer(newServer);
        setShowCreateModal(false);
        setServerName("");
        setServerDescription("");
      }
    } catch (e) { alert(t.chatbar_create_error); }
  };

  const handleJoinServer = async () => {
    if (!joinCode.trim() || !joinServerId.trim()) {
        alert(t.chatbar_join_fields_required);
        return;
    }

    try {
      const token = localStorage.getItem("access_token");
      
      // On utilise l'ID saisi pour construire l'URL
      const res = await fetch(`http://localhost:3000/servers/${joinServerId}/join`, {
        method: "POST",
        headers: { 
            "Content-Type": "application/json", 
            "Authorization": `Bearer ${token}` 
        },
        body: JSON.stringify({ invitcode: parseInt(joinCode) }),
      });

      if (res.ok) {
        await refreshUserData();
        setJoinCode("");
        setJoinServerId("");
        setShowJoinInput(false);
      } else {
        const errorData = await res.json().catch(() => ({}));
        const raw: string = errorData.error || "";
        let msg: string;
        if (raw.toLowerCase().includes("permanently banned")) {
          msg = t.chatbar_join_banned_perm;
        } else if (raw.toLowerCase().includes("temporarily banned")) {
          msg = t.chatbar_join_banned_temp;
        } else if (raw.toLowerCase().includes("already a member")) {
          msg = t.chatbar_join_already_member;
        } else if (raw.toLowerCase().includes("invalid invitation")) {
          msg = t.chatbar_join_invalid_code;
        } else {
          msg = raw || t.chatbar_join_generic_error;
        }
        alert(msg);
      }
    } catch (error: any) {
      console.error(error);
      alert(t.chatbar_join_network_error);
    }
  };

  const handleDeleteServer = async (serverId: number) => {
    if(!confirm(t.chatbar_delete_server_confirm)) return;
    try {
        const token = localStorage.getItem("access_token");
        const res = await fetch(`http://localhost:3000/servers/${serverId}`, {
            method: "DELETE",
            headers: { "Authorization": `Bearer ${token}` },
        });
        if (res.ok) {
            await refreshUserData(); // Rafraîchit les données avec le backend
            if (selectedServerId === serverId) {
                setSelectedServerId(null);
                setSelectedChannelId(null);
                onServerSelect?.(null);
                onChannelSelect?.(null);
            }
        } else {
            alert("Erreur lors de la suppression du serveur.");
        }
    } catch (e) { console.error(e); }
  };

  const handleLeaveServer = async (serverId: number) => {
    if(!confirm(t.chatbar_leave_server_confirm)) return;
    try {
        const token = localStorage.getItem("access_token");
        const res = await fetch(`http://localhost:3000/servers/${serverId}/leave`, {
            method: "DELETE", // ou POST selon votre API
            headers: { "Authorization": `Bearer ${token}` },
        });
        if (res.ok) {
            await refreshUserData(); // Rafraîchit les données avec le backend
            if (selectedServerId === serverId) {
                setSelectedServerId(null);
                setSelectedChannelId(null);
                onServerSelect?.(null);
                onChannelSelect?.(null);
            }
        } else {
             alert("Erreur : impossible de quitter ce serveur.");
        }
    } catch (e) { console.error(e); }
  };

  // --- ACTIONS CHANNEL ---

  const handleCreateChannel = async (serverId: number) => {
    const name = prompt(t.chatbar_channel_name_prompt);
    if (!name) return;
    
    try {
        const token = localStorage.getItem("access_token");
        const res = await fetch(`http://localhost:3000/servers/${serverId}/channels`, {
            method: "POST",
            headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
            body: JSON.stringify({ server_id: serverId, name: name, description: "New channel" }),
        });
        if (res.ok) await refreshUserData();
    } catch (e) { console.error(e); }
  };
  
  const handleDeleteChannel = async (channelId: string) => {
    if (!confirm(t.chatbar_delete_channel_confirm)) return;
    try {
      const token = localStorage.getItem("access_token");
      const res = await fetch(`http://localhost:3000/channels/${channelId}`, {
        method: "DELETE",
        headers: { "Authorization": `Bearer ${token}` },
      });
      if (res.ok) await refreshUserData();
    } catch (e) { console.error(e); }
  };

  // --- ACTIONS CHANNEL (Update) ---

  const startEditingChannel = (channel: Channel, e: React.MouseEvent) => {
      e.stopPropagation();
      setEditingChannelId(channel.id);
      setEditChannelName(channel.name);
  };

  const cancelEditingChannel = (e?: React.MouseEvent) => {
      e?.stopPropagation();
      setEditingChannelId(null);
      setEditChannelName("");
  };

  const handleUpdateChannel = async (channelId: string) => {
      if (!editChannelName.trim()) return;
      const token = localStorage.getItem("access_token");

      try {
          const res = await fetch(`http://localhost:3000/channels/${channelId}`, {
              method: "PUT",
              headers: { 
                  "Content-Type": "application/json", 
                  "Authorization": `Bearer ${token}` 
              },
              body: JSON.stringify({ name: editChannelName })
          });

          if (res.ok) {
              setEditingChannelId(null); 
          } else {
              alert(t.chatbar_edit_error);
          }
      } catch(e) { console.error(e); }
  };

  // --- ECOUTE WEBSOCKET POUR UPDATE CHANNEL ---
  useEffect(() => {
      if (!socket) return;

      const handleWSMessage = (event: MessageEvent) => {
          try {
              const parsed = JSON.parse(event.data);
              
              if (parsed.type === "UPDATE_CHANNEL") {
                  const { server_id, channel_id, name } = parsed.data;
                  
                  // CORRECTION : On calcule la nouvelle liste basée sur 'servers' actuel (déjà dans le scope)
                  // au lieu d'utiliser une fonction callback dans setServers.
                  const updatedServers = servers.map(srv => {
                      if (srv.id === server_id) {
                          return {
                              ...srv,
                              channels: srv.channels?.map(ch => 
                                  ch.id === channel_id ? { ...ch, name: name } : ch
                              )
                          };
                      }
                      return srv;
                  });

                  setServers(updatedServers);
              }

              // GESTION DU DÉPART D'UN UTILISATEUR
              if (parsed.type === "user_left") {
                  const { server_id, user_id } = parsed.data;

                  setServers(servers.map(s => {
                      if (s.id === server_id) {
                          return {
                              ...s,
                              // On met à jour la liste des membres (objets) et des admins (IDs)
                              // On utilise 'any' sur m car l'interface Server définit members comme string[] par erreur
                              // alors que le backend envoie des objets MemberItem complets.
                              members: s.members?.filter((m: any) => m.user_id !== user_id),
                              admins: s.admins?.filter(id => id !== user_id)
                          };
                      }
                      return s;
                  }));
              }
          } catch (e) { console.error("WS Error Chatbar", e); }
      };

      socket.addEventListener("message", handleWSMessage);
      return () => socket.removeEventListener("message", handleWSMessage);
  }, [socket, servers, setServers]); // On ajoute 'servers' aux dépendances


  const [editingChannelId, setEditingChannelId] = useState<string | null>(null);
  const [editChannelName, setEditChannelName] = useState("");

  return (
    <div className="fixed top-16 h-[calc(100vh-4rem)] w-64 bg-[#001839] border-l border-[#3D3D3D] flex flex-col p-4 z-10 shadow-lg">
      
      {/* --- ONGLET NAVIGATION --- */}
      <div className="flex bg-[#0F0F1A] rounded-lg p-1 mb-4 border border-[#3D3D3D]">
        <button
          onClick={() => setActiveTab("servers")}
          className={`flex-1 py-1.5 text-sm font-bold rounded-md transition ${
            activeTab === "servers" ? "bg-blue-600 text-white shadow" : "text-gray-400 hover:text-white hover:bg-white/5"
          }`}
        >
          Serveurs
        </button>
        <button
          onClick={() => setActiveTab("dms")}
          className={`flex-1 py-1.5 text-sm font-bold rounded-md transition ${
            activeTab === "dms" ? "bg-blue-600 text-white shadow" : "text-gray-400 hover:text-white hover:bg-white/5"
          }`}
        >
          DMs
        </button>
      </div>

      {/* ========================================= */}
      {/* VUE SERVEURS */}
      {/* ========================================= */}
      {activeTab === "servers" && (
        <>
          {/* --- ZONE CRÉATION / JOIN --- */}
          <div className="mb-4 space-y-2">
            <h2 className="text-white text-lg font-bold mb-2">Serveurs</h2>
            
            <button 
                onClick={() => setShowCreateModal(true)} 
                className="w-full py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition shadow-md"
            >
              Créer un Serveur
            </button>
            
            <button 
                onClick={() => setShowJoinInput(!showJoinInput)} 
                className="w-full py-2 bg-[#2A2A3D] hover:bg-[#3D3D5C] text-white rounded-lg font-medium transition border border-gray-600"
            >
              Rejoindre un Serveur
            </button>

            {showJoinInput && (
              <div className="mt-2 flex flex-col gap-2 p-2 bg-[#1E1E2E] rounded border border-gray-700 animate-in fade-in slide-in-from-top-2">
                <input 
                    type="number" 
                    value={joinServerId} 
                    onChange={(e) => setJoinServerId(e.target.value)} 
                    placeholder="ID du serveur" 
                    className="w-full p-2 rounded bg-[#0F0F1A] text-white text-sm outline-none border border-gray-600 focus:border-blue-500" 
                />
                <input 
                    type="number" 
                    value={joinCode} 
                    onChange={(e) => setJoinCode(e.target.value)} 
                    placeholder="Code invitation (4 chiffres)" 
                    className="w-full p-2 rounded bg-[#0F0F1A] text-white text-sm outline-none border border-gray-600 focus:border-blue-500" 
                />
                <button 
                    onClick={handleJoinServer} 
                    className="w-full bg-green-600 hover:bg-green-700 py-1.5 rounded text-white text-sm font-bold transition"
                >
                    Confirmer
                </button>
              </div>
            )}
          </div>

          <hr className="border-[#3D3D3D] mb-4" />

          {/* --- LISTE DES SERVEURS --- */}
          <div className="flex-1 overflow-y-auto space-y-3 pr-1 scrollbar-thin scrollbar-thumb-gray-600">
            {servers.map((server: Server) => {
              const isExpanded = expandedServerId === server.id;
              const serverChannels = server.channels || [];
              
              // Vérification visuelle basique : Est-ce que je suis l'owner ?
              // (Pour une gestion Admin complète, il faudrait vérifier les rôles via l'API)
              const canEditElement = user?.id === server.owner_id;

              return (
                <div key={server.id} className="group flex flex-col">
                  <div 
                    onClick={() => { 
                      setSelectedServerId(server.id); 
                      onServerSelect?.(server); 
                    }}
                    className={`flex items-center justify-between p-3 rounded-lg cursor-pointer transition-all border border-transparent ${selectedServerId === server.id ? "bg-blue-600 text-white shadow-md" : "bg-[#1E1E2E] text-gray-300 hover:bg-[#2A2A3D] hover:border-gray-600"}`}
                  >
                    <div className="flex items-center gap-2 overflow-hidden flex-1">
                      <button 
                        // MODIFICATION 1 : La flèche sélectionne AUSSI le serveur maintenant
                        onClick={(e) => { 
                            e.stopPropagation(); 
                            setExpandedServerId(isExpanded ? null : server.id);
                            // On force la sélection du serveur
                            setSelectedServerId(server.id);
                            onServerSelect?.(server);
                        }}
                        className="w-5 h-5 flex items-center justify-center hover:bg-white/20 rounded transition-transform text-xs"
                        style={{ transform: isExpanded ? 'rotate(90deg)' : 'rotate(0deg)' }}
                      >
                        ▶
                      </button>
                      <span className="font-bold truncate">{server.name}</span>
                    </div>
                    {/* ID affiché au survol pour aider le copain */}
                    <span title={`ID: ${server.id} | Code: ${server.invitcode}`} className="text-[10px] text-gray-500 mr-2 opacity-0 group-hover:opacity-100 cursor-help">ℹ️</span>

                    {/* LOGIQUE BOUTONS : SUPPRIMER vs QUITTER */}
                    {user?.id === server.owner_id ? (
                        <button 
                            onClick={(e) => { e.stopPropagation(); handleDeleteServer(server.id); }} 
                            className="opacity-0 group-hover:opacity-100 text-red-400 hover:text-red-500 hover:bg-white/10 p-1 rounded transition-all"
                            title="Supprimer le serveur"
                        >
                            🗑️
                        </button>
                    ) : (
                        <button 
                            onClick={(e) => { e.stopPropagation(); handleLeaveServer(server.id); }} 
                            className="opacity-0 group-hover:opacity-100 text-yellow-400 hover:text-yellow-500 hover:bg-white/10 p-1 rounded transition-all text-xs font-bold"
                            title="Quitter le serveur"
                        >
                            🚪
                        </button>
                    )}
                  </div>

                  {/* LISTE DES CHANNELS */}
                  {isExpanded && (
                    <div className="mt-1 py-2 px-2 bg-[#0F0F1A] rounded-b-lg border-x border-b border-[#3D3D3D] space-y-1 ml-2 border-l-2 border-l-blue-600">
                      {serverChannels.length === 0 && <p className="text-xs text-gray-500 text-center py-1">Aucun salon</p>}
                      
                      {serverChannels.map(chan => {
                        
                        // NOUVELLE LOGIQUE PERMISSION CRAYON
                        // Owner OU Admin (si la liste admins est présente et contient mon ID)
                        const isAdmin = server.admins?.includes(user?.id || "");
                        const canEditChannel = (user?.id === server.owner_id) || isAdmin;

                        return (
                            <div 
                                key={chan.id} 
                                onClick={() => { setSelectedChannelId(chan.id); onChannelSelect?.(chan); }}
                                className={`flex items-center justify-between group/chan px-2 py-1.5 rounded cursor-pointer transition-colors ${selectedChannelId === chan.id ? "bg-blue-900/50 text-blue-200 border-l-2 border-blue-500" : "hover:bg-[#1E1E2E] text-gray-400 hover:text-gray-200 border-l-2 border-transparent"}`}
                            >
                          {/* NOM DU SALON OU INPUT D'EDITION */}
                          {editingChannelId === chan.id ? (
                              <input 
                                  autoFocus
                                  type="text" 
                                  value={editChannelName}
                                  onChange={(e) => setEditChannelName(e.target.value)}
                                  onClick={(e) => e.stopPropagation()} // Bloque la sélection du salon pendant l'écriture
                                  onKeyDown={(e) => {
                                      if (e.key === "Enter") handleUpdateChannel(chan.id);
                                      if (e.key === "Escape") cancelEditingChannel();
                                  }}
                                  className="w-full bg-black/50 text-white text-xs px-1 py-0.5 rounded border border-blue-500 outline-none"
                              />
                          ) : (
                              <span className="text-sm font-medium flex items-center gap-1 truncate">
                                 <span className="text-gray-600">#</span> {chan.name}
                          </span>
                          )}

                          {/* BOUTONS ACTIONS (Cachés par défaut, visibles au survol) */}
                          <div className="flex items-center opacity-0 group-hover/chan:opacity-100 transition-opacity">
                             {/* CRAYON (Seulement si Owner/Admin & Pas en mode édition) */}
                             {canEditChannel && editingChannelId !== chan.id && (
                                 <button
                                    onClick={(e) => startEditingChannel(chan, e)}
                                    className="text-gray-400 hover:text-white mr-1 p-0.5"
                                    title="Modifier"
                                 >
                                    ✏️
                                 </button>
                             )}
                                
                             {/* CROIX (Delete) */}
                             {editingChannelId !== chan.id ? (
                                // MODIFICATION : On vérifie les droits avant d'afficher le bouton supprimer
                                canEditChannel && (
                                    <button 
                                        onClick={(e) => { e.stopPropagation(); handleDeleteChannel(chan.id); }} 
                                        className="text-red-400 hover:text-red-500 text-xs px-1"
                                        title="Supprimer"
                                    >
                                        ✕
                                    </button>
                                )
                             ) : (
                                 // Boutons Validation pendant l'édition
                                 <>
                                    <button onClick={(e) => { e.stopPropagation(); handleUpdateChannel(chan.id); }} className="text-green-500 text-[10px] mr-1">✔</button>
                                    <button onClick={(e) => cancelEditingChannel(e)} className="text-red-500 text-[10px]">✘</button>
                                 </>
                             )}
                          </div>
                        </div>
                      );
                    })}
                      
                      <div className="pt-2 mt-1 border-t border-white/5 flex justify-center">
                        {/* On réutilise la même logique kanEditChannel car souvent les droits sont les mêmes CRUD */}
                        {((user?.id === server.owner_id) || (server.admins?.includes(user?.id || ""))) && (
                            <button 
                                onClick={() => handleCreateChannel(server.id)} 
                                className="text-[10px] text-gray-400 hover:text-white uppercase font-bold tracking-wider hover:underline"
                            >
                        + Nouveau Salon
                        </button>
                    )}
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
      </>)}
      {/* MODAL CRÉATION */}
      {showCreateModal && (
        <div className="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50 animate-in fade-in">
          <div className="bg-[#1E1E2E] p-6 rounded-xl w-96 border border-[#3D3D3D] shadow-2xl">
            <h3 className="text-white text-xl font-bold mb-4">{t.chatbar_modal_new_server}</h3>
            
            <label className="text-xs text-gray-400 uppercase font-bold mb-1 block">{t.chatbar_modal_name}</label>
            <input 
                type="text" 
                className="w-full mb-4 p-2 rounded bg-[#0F0F1A] text-white border border-[#3D3D3D] focus:border-blue-500 outline-none transition" 
                value={serverName}
                onChange={e => setServerName(e.target.value)} 
            />
            
            <label className="text-xs text-gray-400 uppercase font-bold mb-1 block">{t.chatbar_modal_description}</label>
            <textarea 
                className="w-full mb-6 p-2 rounded bg-[#0F0F1A] text-white border border-[#3D3D3D] focus:border-blue-500 outline-none h-24 resize-none transition" 
                value={serverDescription}
                onChange={e => setServerDescription(e.target.value)} 
            />
            
            <div className="flex justify-between items-center bg-[#0F0F1A] -m-6 mt-0 p-4 rounded-b-xl border-t border-[#3D3D3D]">
               <button onClick={() => setShowCreateModal(false)} className="text-gray-400 hover:text-white text-sm font-medium transition">{t.chatbar_modal_cancel}</button>
               <button onClick={handleCreateServer} className="bg-blue-600 text-white px-6 py-2 rounded hover:bg-blue-700 font-bold shadow-lg transition">{t.chatbar_modal_create}</button>
            </div>
          </div>
        </div>
      )}
    </div>
    </>
  );
}
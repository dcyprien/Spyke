"use client";

import React, { useEffect, useRef, useState } from "react";
import { Server, Channel } from "./chatbar"; 
import { useAuth } from "../app/context";
import { useLang } from "../app/langContext";

type Message = {
  id: string; 
  author: string;
  author_id: string;
  content: string;
  time: string;
  serverId: number; 
  channelId: string;
};

type Props = {
  selectedServer?: Server | null;
  selectedChannel?: Channel | null;
  mobileTab?: string;
  activeDMUser?: { id: string; name: string } | null; // <-- AJOUT
};

export default function Chat({ selectedServer, selectedChannel, mobileTab, activeDMUser }: Props) {
  const { user, socket } = useAuth();
  const { t } = useLang();
  
  // États de base
  const [message, setMessage] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [showEmojiPicker, setShowEmojiPicker] = useState(false);
  const [typingUsers, setTypingUsers] = useState<string[]>([]);
  
  // États UI
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);

  // --- NOUVEAUX ÉTATS POUR L'ÉDITION ---
  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [editContent, setEditContent] = useState("");
  // -------------------------------------

  const isTypingRef = useRef(false);
  const typingTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const shouldAutoScroll = useRef(true);

  const emojis = ["😀", "😃", "😄", "😁", "😆", "😅", "😂", "❤️", "🔥", "👍", "✨"];

  // Fermer le menu si on clique ailleurs
  useEffect(() => {
    const handleClickOutside = () => setOpenMenuId(null);
    document.addEventListener("click", handleClickOutside);
    return () => document.removeEventListener("click", handleClickOutside);
  }, []);

  // Bloque scroll body
  useEffect(() => {
    document.documentElement.style.overflow = "hidden";
    document.body.style.overflow = "hidden";
    return () => {
      document.documentElement.style.overflow = "";
      document.body.style.overflow = "";
    };
  }, []);

  // CHARGEMENT INITIAL
  useEffect(() => {
    // Si ni channel ni DM sélectionné, on arrête
    if (!selectedChannel && !activeDMUser) {
        setMessages([]);
        return;
    }

    const fetchMessages = async () => {
        const token = localStorage.getItem("access_token");
        try {
            let res;
            if (selectedChannel) {
                res = await fetch(`http://localhost:3000/channels/${selectedChannel.id}/messages`, {
                    headers: { "Authorization": `Bearer ${token}` }
                });
            } else if (activeDMUser) {
                // ATTENTION: Remplacez l'URL par l'endpoint réel de vos DMs dans votre backend
                res = await fetch(`http://localhost:3000/dms/${activeDMUser.id}/messages`, {
                    headers: { "Authorization": `Bearer ${token}` }
                });
            }

            if (res && res.ok) {
                const data = await res.json();
                const history = data.message_list.map((msg: any) => ({
                    id: String(msg.id),
                    author: msg.author || "Utilisateur",
                    author_id: msg.user_id,
                    content: msg.content,
                    time: new Date(msg.created_at).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
                    serverId: msg.server_id,
                    channelId: msg.channel_id,
                }));
                setMessages(history);
            }
        } catch (e) { console.error(e); }
    };
    fetchMessages();
    setEditingMessageId(null); // Reset edit si on change de channel
  }, [selectedChannel, activeDMUser]); // <-- Mettre à jour les dépendances


  // ÉCOUTE DU WEBSOCKET
  useEffect(() => {
    if (!socket || !selectedChannel) return;

    const handleMessage = (event: MessageEvent) => {
        try {
            const parsed = JSON.parse(event.data);
            const data = parsed.data;

            if (parsed.type === "DELETE_MESSAGE" || parsed.type === "UPDATE_MESSAGE") {
                console.log("WS Event reçu:", parsed.type, data);
            }

            switch (parsed.type) {
                case "new_message":             
                    if (String(data.channel_id) === String(selectedChannel.id)) {
                        const newMsg: Message = {
                            id: String(data.id),
                            author: data.author_username || data.user_id,
                            author_id: data.user_id,
                            content: data.content,
                            time: new Date(data.created_at).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
                            serverId: data.server_id,
                            channelId: data.channel_id
                        };
                        setMessages((prev) => [...prev, newMsg]);
                    }
                    break;
                
                case "DELETE_MESSAGE":
                    if (String(data.channel_id) === String(selectedChannel.id)) {
                        setMessages((prev) => prev.filter(m => String(m.id) !== String(data.message_id)));
                    }
                    break;

                // --- AJOUT : MISE À JOUR DU MESSAGE ---
                case "UPDATE_MESSAGE":
                    if (String(data.channel_id) === String(selectedChannel.id)) {
                        setMessages((prev) => prev.map(m => {
                            if (String(m.id) === String(data.message_id)) {
                                return { ...m, content: data.new_content };
                            }
                            return m;
                        }));
                    }
                    break;
                // -------------------------------------

                case "typing_start":
                    if (data.username !== user?.username && String(data.channel_id) === String(selectedChannel.id)) {
                        setTypingUsers(prev => {
                            if (!prev.includes(data.username)) return [...prev, data.username];
                            return prev;
                        });
                    }
                    break;

                case "typing_stop":
                    if (String(data.channel_id) === String(selectedChannel.id)) {
                        setTypingUsers(prev => prev.filter(u => u !== data.username));
                    }
                    break;
            }
        } catch (e) { 
            console.error("WS Parse Error inside Chat", e);
        }
    };

    socket.addEventListener("message", handleMessage);
    return () => socket.removeEventListener("message", handleMessage);
  }, [socket, selectedChannel, user]);


  const sendMessage = async () => {
    // On bloque si on n'a ni channel ni DM
    if (!message.trim() || (!selectedChannel && !activeDMUser)) return;

    const token = localStorage.getItem("access_token");
    const contentToSend = message;
    setMessage(""); 

    try {
        if (selectedChannel && selectedServer) {
            await fetch(`http://localhost:3000/channels/${selectedChannel.id}/messages`, {
                method: "POST",
                headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
                body: JSON.stringify({ server_id: selectedServer.id, content: contentToSend })
            });
            if (socket){
                socket.send(JSON.stringify({ type: "typing_stop", server_id: selectedServer.id, channel_id: selectedChannel.id }));
            }
        } else if (activeDMUser) {
            // ATTENTION: Endpoint DM à adapter selon votre backend
            await fetch(`http://localhost:3000/dms/${activeDMUser.id}/messages`, {
                method: "POST",
                headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
                body: JSON.stringify({ content: contentToSend })
            });
        }
    } catch (e) { console.error(e); }
  };

  const deleteMessage = async (msgId: string) => {
    const token = localStorage.getItem("access_token");
    setMessages((prev) => prev.filter(m => m.id !== msgId)); // Optimistic

    try {
        await fetch(`http://localhost:3000/messages/${msgId}`, {
            method: "DELETE",
            headers: { "Authorization": `Bearer ${token}` }
        });
    } catch (e) { console.error(e); }
  };

  // --- NOUVELLES FONCTIONS EDITION ---
  
  const startEditing = (msg: Message) => {
      setEditingMessageId(msg.id);
      setEditContent(msg.content);
      setOpenMenuId(null); // Fermer le menu
  };

  const cancelEditing = () => {
      setEditingMessageId(null);
      setEditContent("");
  };

  const saveEdit = async (msgId: string) => {
      if (!editContent.trim()) return cancelEditing();
      
      const token = localStorage.getItem("access_token");

      // Optimistic UI update (Optionnel, mais agréable)
      setMessages(prev => prev.map(m => m.id === msgId ? { ...m, content: editContent } : m));
      setEditingMessageId(null);

      try {
          const res = await fetch(`http://localhost:3000/messages/${msgId}`, {
              method: "PUT",
              headers: { 
                  "Content-Type": "application/json",
                  "Authorization": `Bearer ${token}` 
              },
              body: JSON.stringify({ new_content: editContent })
          });

          if (!res.ok) {
              // Revert si erreur (simple reload ou revert manual)
              const err = await res.json();
              alert(err.error || "Erreur modification");
          }
           // Si succès, le WebSocket BROADCAST confirmera à tout le monde (y compris nous)
      } catch (e) {
          console.error("Erreur update", e);
      }
  };
  // -----------------------------------

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setMessage(val);
    if (socket && selectedChannel && selectedServer && user) {
        if (!isTypingRef.current && val.length > 0) {
            isTypingRef.current = true;
            socket.send(JSON.stringify({
                type: "typing_start",
                server_id: selectedServer.id,
                channel_id: selectedChannel.id
            }));
        }
        if (typingTimeoutRef.current) clearTimeout(typingTimeoutRef.current);

        if (val.length === 0) {
            socket.send(JSON.stringify({
                type: "typing_stop",
                server_id: selectedServer.id,
                channel_id: selectedChannel.id
            }));
            isTypingRef.current = false;
        } else {
            typingTimeoutRef.current = setTimeout(() => {
                if (isTypingRef.current) {
                    socket.send(JSON.stringify({
                        type: "typing_stop",
                        server_id: selectedServer.id,
                        channel_id: selectedChannel.id
                    }));
                    isTypingRef.current = false;
                }
            }, 3000);
        }
    }
  };
  const handleScroll = () => {
    if (messagesRef.current) {
        const { scrollTop, scrollHeight, clientHeight } = messagesRef.current;
        shouldAutoScroll.current = scrollTop + clientHeight >= scrollHeight * 0.9;
    }
  };
  useEffect(() => {
    if (messagesRef.current && shouldAutoScroll.current) {
      messagesRef.current.scrollTo({ top: messagesRef.current.scrollHeight, behavior: "smooth" });
    }
  }, [messages]);


  return (
    <div className={`flex flex-col h-screen bg-[#001952]
      pt-16 pb-16 md:pt-20 md:pb-0 px-4
      md:ml-64 lg:mr-64
      ${mobileTab !== "chat" ? "hidden md:flex" : "flex"}
    `}>
      {/* Header */}
      <div className="py-3 border-b border-white/10 mb-4 flex items-center justify-center">
        <h3 className="text-white font-bold text-lg">
          {selectedChannel 
            ? `# ${selectedChannel.name}` 
            : activeDMUser 
              ? `💬 Message privé avec ${activeDMUser.name}` 
              : t.chat_select_channel}
        </h3>
        {selectedServer && <span className="text-blue-gray text-xs bg-dark-navy px-2 py-1 rounded ml-2">{selectedServer.name}</span>}
      </div>

      {/* Historique */}
      <div ref={messagesRef} onScroll={handleScroll} className="flex-1 space-y-4 mb-4 overflow-y-auto custom-scrollbar px-2">
        {!selectedChannel && !activeDMUser ? (
           <div className="text-center text-blue-gray mt-10">{t.chat_select_channel}</div>
        ) : messages.length === 0 ? (
          <div className="text-center text-blue-gray py-8 italic">{t.chat_no_messages}</div>
        ) : (
          messages.map((msg) => (
            <div 
              key={msg.id} 
              className={`max-w-xl mx-auto group relative ${openMenuId === msg.id ? "z-50" : "z-auto"}`}
            >
              <div className="text-[10px] text-blue-gray mb-1 flex justify-between items-end px-1">
                <span className={`font-bold ${msg.author_id === user?.id ? "text-green-400" : "text-cyan"}`}>
                    {msg.author}
                </span>
                <span className="opacity-70">{msg.time}</span>
              </div>

              {/* Bulle Message : MODE NORMAL vs MODE ÉDITION */}
              <div className="bg-dark-navy text-white px-4 py-2 rounded-lg border border-white/5 break-words relative pr-10">
                
                {/* --- CONTENU DU MESSAGE OU INPUT D'EDITION --- */}
                {editingMessageId === msg.id ? (
                    <div className="flex flex-col gap-2">
                        <input 
                            autoFocus
                            type="text" 
                            className="bg-black/30 border border-blue-500 rounded px-2 py-1 text-sm text-white w-full outline-none"
                            value={editContent}
                            onChange={(e) => setEditContent(e.target.value)}
                            onKeyDown={(e) => {
                                if (e.key === 'Enter') saveEdit(msg.id);
                                if (e.key === 'Escape') cancelEditing();
                            }}
                        />
                        <div className="flex gap-2 text-[10px]">
                            <span className="text-gray-400">Entrée pour valider • Echap pour annuler</span>
                        </div>
                    </div>
                ) : (
                    <>
                        {msg.content} 
                        {/* Indicateur (modifié) si besoin, mais pas stocké en DB dans cet exemple */}
                    </>
                )}


                {/* --- BOUTON OPTIONS (Caché si mode édition) --- */}
                {editingMessageId !== msg.id && (
                    <button
                        onClick={(e) => {
                            e.preventDefault(); e.nativeEvent.stopImmediatePropagation(); e.stopPropagation();
                            setOpenMenuId(prev => prev === msg.id ? null : msg.id);
                        }}
                        className={`absolute top-2 right-2 p-1 rounded hover:bg-white/10 text-gray-400 hover:text-white transition ${openMenuId === msg.id ? "opacity-100 bg-white/10" : "opacity-0 group-hover:opacity-100"}`}
                    >
                        ⋮
                    </button>
                )}

                {/* --- MENU DEROULANT --- */}
                {openMenuId === msg.id && (
                    <div className="absolute top-8 right-0 bg-[#0F0F1A] border border-gray-700 rounded shadow-xl z-50 w-32 py-1 overflow-visible">
                        
                        {/* BOUTON MODIFIER - Uniquement pour l'auteur */}
                        {msg.author_id === user?.id && (
                            <button
                                onClick={(e) => {
                                    e.nativeEvent.stopImmediatePropagation(); e.stopPropagation();
                                    startEditing(msg);
                                }}
                                className="w-full text-left px-4 py-2 text-xs text-gray-300 hover:bg-white/10 hover:text-white flex items-center gap-2"
                            >
                        <span>✏️</span> {t.chat_edit}
                            </button>
                        )}

                        <button
                            onClick={(e) => {
                                e.nativeEvent.stopImmediatePropagation(); e.stopPropagation();
                                deleteMessage(msg.id);
                                setOpenMenuId(null);
                            }}
                            // Conditionner delete si admin peut aussi
                            className="w-full text-left px-4 py-2 text-xs text-red-500 hover:bg-red-500/10 hover:text-red-400 flex items-center gap-2 border-t border-gray-700 mt-1 pt-2"
                        >
                            <span>🗑️</span> {t.chat_delete}
                        </button>
                    </div>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Indicateur de frappe */}
      {typingUsers.length > 0 && (
        <div className="max-w-xl mx-auto w-full px-2 text-xs text-cyan italic mb-1">
          {typingUsers.length === 1 
            ? `${typingUsers[0]} est en train d'écrire...` 
            : `${typingUsers.join(", ")} sont en train d'écrire...`}
        </div>
      )}

      {/* Input de saisie principal (Inchangé) */}
      <div className="max-w-xl mx-auto w-full flex items-center gap-2 pb-6 relative">
          {/* ... emoji picker ... */}
         {/* ... input ... */}
         {/* Je ne remets pas tout le code bas de page car il ne change pas, 
             référez-vous au code existant pour le return final de l'input et du bouton envoyer */}
        <div className="relative">
          <button 
            onClick={() => setShowEmojiPicker(!showEmojiPicker)}
            disabled={!selectedChannel && !activeDMUser}
            className="p-2 rounded-lg bg-dark-navy hover:bg-navy-deep text-xl disabled:opacity-30 transition"
          >
            😊
          </button>
          {showEmojiPicker && (
            <div className="absolute bottom-14 left-0 bg-dark-navy rounded-lg p-2 border border-grey shadow-2xl z-50 grid grid-cols-5 gap-1 w-40">
              {emojis.map((emoji, idx) => (
                <button
                  key={idx}
                  onClick={() => { setMessage(p => p + emoji); setShowEmojiPicker(false); }}
                  className="hover:bg-blue-mid rounded p-1"
                >
                  {emoji}
                </button>
              ))}
            </div>
          )}
        </div>

        <input
          type="text"
          value={message}
          onChange={handleInputChange}
          onKeyDown={(e) => { if(e.key === "Enter") { sendMessage(); } }}
          placeholder={
            selectedChannel 
              ? `${t.chat_send_placeholder} # ${selectedChannel.name}` 
              : activeDMUser 
                ? `Envoyer un message à ${activeDMUser.name}...` 
                : "..."
          }
          disabled={!selectedChannel && !activeDMUser}
          className="flex-1 min-w-0 bg-dark-navy text-white px-3 py-2 rounded-lg outline-none border border-white/10 focus:border-cyan disabled:opacity-30 text-sm"
        />
        <button
          onClick={sendMessage}
          disabled={!selectedChannel && !activeDMUser}
          className="bg-cyan hover:bg-blue-mid text-white px-3 sm:px-5 py-2 rounded-lg font-bold disabled:opacity-30 transition shadow-lg flex-shrink-0 flex items-center gap-1 text-sm"
        >
          <svg className="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
          </svg>
          <span className="hidden sm:inline">{t.chat_send_btn}</span>
        </button>

      </div>
    </div>
  );
}
"use client";

import React, { useEffect, useRef, useState } from "react";
import { Server, Channel } from "./chatbar"; 
import { useAuth } from "../app/context";
import { useLang } from "../app/langContext";

type Reaction = {
  emoji: string;
  userIds: string[];
};

type Message = {
  id: string; 
  author: string;
  author_id: string;
  content: string;
  time: string;
  serverId: number; 
  channelId: string;
  reactions: Reaction[];
};

const REACTION_EMOJIS = ["👍", "👎", "😊", "😢", "❤️", "😂"];

type Props = {
  selectedServer?: Server | null;
  selectedChannel?: Channel | null;
  mobileTab?: string;
};

export default function Chat({ selectedServer, selectedChannel, mobileTab }: Props) {
  const { user, socket } = useAuth();
  const { t } = useLang();
  
  // États de base
  const [message, setMessage] = useState("");
  const [messages, setMessages] = useState<Message[]>([]);
  const [showEmojiPicker, setShowEmojiPicker] = useState(false);
  const [typingUsers, setTypingUsers] = useState<string[]>([]);
  
  // États UI
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const [reactionPickerId, setReactionPickerId] = useState<string | null>(null);

  // --- NOUVEAUX ÉTATS POUR L'ÉDITION ---
  const [editingMessageId, setEditingMessageId] = useState<string | null>(null);
  const [editContent, setEditContent] = useState("");
  // -------------------------------------

  // --- ÉTATS GIF PICKER ---
  const [showGifPicker, setShowGifPicker] = useState(false);
  const [gifSearch, setGifSearch] = useState("");
  const [gifResults, setGifResults] = useState<{ id: string; url: string; preview: string }[]>([]);
  const [gifLoading, setGifLoading] = useState(false);
  // ------------------------

  const isTypingRef = useRef(false);
  const typingTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const messagesRef = useRef<HTMLDivElement | null>(null);
  const shouldAutoScroll = useRef(true);
  const gifPickerRef = useRef<HTMLDivElement | null>(null);
  const gifBtnRef = useRef<HTMLButtonElement | null>(null);
  const [gifPickerPos, setGifPickerPos] = useState({ bottom: 0, right: 0 });

  const emojis = ["😀", "😃", "😄", "😁", "😆", "😅", "😂", "❤️", "🔥", "👍", "✨"];

  // Fermer le menu / reaction picker si on clique ailleurs
  useEffect(() => {
    const handleClickOutside = () => { setOpenMenuId(null); setReactionPickerId(null); };
    document.addEventListener("click", handleClickOutside);
    return () => document.removeEventListener("click", handleClickOutside);
  }, []);

  // Fermer le GIF picker si clic en dehors (mousedown pour éviter les conflits React)
  useEffect(() => {
    if (!showGifPicker) return;
    const handler = (e: MouseEvent) => {
      if (
        gifPickerRef.current && !gifPickerRef.current.contains(e.target as Node) &&
        gifBtnRef.current && !gifBtnRef.current.contains(e.target as Node)
      ) {
        setShowGifPicker(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showGifPicker]);

  // Fetch GIFs (trending ou recherche)
  const fetchGifs = async (query: string) => {
    const apiKey = process.env.NEXT_PUBLIC_GIPHY_API_KEY;
    setGifLoading(true);
    try {
      const endpoint = query.trim()
        ? `https://api.giphy.com/v1/gifs/search?api_key=${apiKey}&q=${encodeURIComponent(query)}&limit=24&rating=g`
        : `https://api.giphy.com/v1/gifs/trending?api_key=${apiKey}&limit=24&rating=g`;
      const res = await fetch(endpoint);
      if (res.ok) {
        const data = await res.json();
        setGifResults(data.data.map((g: any) => ({
          id: g.id,
          url: g.images.fixed_height.url,
          preview: g.images.fixed_height_small.url,
        })));
      }
    } catch (e) { console.error(e); }
    setGifLoading(false);
  };

  // Debounce recherche GIF
  useEffect(() => {
    if (!showGifPicker) return;
    const timeout = setTimeout(() => fetchGifs(gifSearch), 350);
    return () => clearTimeout(timeout);
  }, [gifSearch, showGifPicker]);

  const sendGif = async (gifUrl: string) => {
    if (!selectedServer || !selectedChannel) return;
    const token = localStorage.getItem("access_token");
    setShowGifPicker(false);
    try {
      await fetch(`http://localhost:3000/channels/${selectedChannel.id}/messages`, {
        method: "POST",
        headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
        body: JSON.stringify({ server_id: selectedServer.id, content: gifUrl }),
      });
    } catch (e) { console.error(e); }
  };

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
    if (!selectedChannel) return;

    const fetchMessages = async () => {
        const token = localStorage.getItem("access_token");
        try {
            const res = await fetch(`http://localhost:3000/channels/${selectedChannel.id}/messages`, {
                headers: { "Authorization": `Bearer ${token}` }
            });
            if (res.ok) {
                const data = await res.json();
                const history = data.message_list.map((msg: any) => ({
                    id: String(msg.id),
                    author: msg.author || "Utilisateur",
                    author_id: msg.user_id,
                    content: msg.content,
                    time: new Date(msg.created_at).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
                    serverId: msg.server_id,
                    channelId: msg.channel_id,
                    reactions: (msg.reactions || []).map((r: any) => ({ emoji: r.emoji, userIds: r.user_ids || [] })),
                }));
                setMessages(history);
            }
        } catch (e) { console.error(e); }
    };
    fetchMessages();
    setEditingMessageId(null); // Reset edit si on change de channel
  }, [selectedChannel]);


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
                            channelId: data.channel_id,
                            reactions: [],
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

                case "REACTION_ADD":
                    if (String(data.channel_id) === String(selectedChannel.id)) {
                        setMessages((prev) => prev.map(m => {
                            if (String(m.id) !== String(data.message_id)) return m;
                            const existing = m.reactions.find(r => r.emoji === data.emoji);
                            if (existing) {
                                if (existing.userIds.includes(data.user_id)) return m; // optimistic already applied
                                return { ...m, reactions: m.reactions.map(r => r.emoji === data.emoji ? { ...r, userIds: [...r.userIds, data.user_id] } : r) };
                            }
                            return { ...m, reactions: [...m.reactions, { emoji: data.emoji, userIds: [data.user_id] }] };
                        }));
                    }
                    break;

                case "REACTION_REMOVE":
                    if (String(data.channel_id) === String(selectedChannel.id)) {
                        setMessages((prev) => prev.map(m => {
                            if (String(m.id) !== String(data.message_id)) return m;
                            const existing = m.reactions.find(r => r.emoji === data.emoji);
                            if (!existing) return m;
                            const updated = existing.userIds.filter((id: string) => id !== data.user_id);
                            return {
                                ...m,
                                reactions: updated.length === 0
                                    ? m.reactions.filter(r => r.emoji !== data.emoji)
                                    : m.reactions.map(r => r.emoji === data.emoji ? { ...r, userIds: updated } : r)
                            };
                        }));
                    }
                    break;

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
    if (!message.trim() || !selectedServer || !selectedChannel) return;

    const token = localStorage.getItem("access_token");
    const contentToSend = message;
    setMessage(""); 

    try {
        await fetch(`http://localhost:3000/channels/${selectedChannel.id}/messages`, {
            method: "POST",
            headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
            body: JSON.stringify({ server_id: selectedServer.id, content: contentToSend })
        });
        setMessage(""); 
    } catch (e) { console.error(e); }
    if (socket){
        socket.send(JSON.stringify({ type: "typing_stop", server_id: selectedServer.id, channel_id: selectedChannel.id }));
    }
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

  const toggleReaction = async (msgId: string, emoji: string) => {
    if (!user) return;
    // Optimistic update
    setMessages(prev => prev.map(m => {
      if (m.id !== msgId) return m;
      const existing = m.reactions.find(r => r.emoji === emoji);
      let newReactions: Reaction[];
      if (existing) {
        const hasReacted = existing.userIds.includes(user.id);
        if (hasReacted) {
          const updated = existing.userIds.filter(id => id !== user.id);
          newReactions = updated.length === 0
            ? m.reactions.filter(r => r.emoji !== emoji)
            : m.reactions.map(r => r.emoji === emoji ? { ...r, userIds: updated } : r);
        } else {
          newReactions = m.reactions.map(r => r.emoji === emoji ? { ...r, userIds: [...r.userIds, user.id] } : r);
        }
      } else {
        newReactions = [...m.reactions, { emoji, userIds: [user.id] }];
      }
      return { ...m, reactions: newReactions };
    }));
    setReactionPickerId(null);
    // Persist to backend (WebSocket will sync other users)
    const token = localStorage.getItem("access_token");
    try {
      await fetch(`http://localhost:3000/messages/${msgId}/reactions`, {
        method: "PUT",
        headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
        body: JSON.stringify({ emoji }),
      });
    } catch (e) { console.error(e); }
  };

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


  const isGifUrl = (content: string) => {
    const trimmed = content.trim();
    return trimmed.startsWith("https://media") && trimmed.includes("giphy.com");
  };

  return (
    <div className={`flex flex-col h-screen bg-[#001952]
      pt-16 pb-16 md:pt-20 md:pb-0 px-4
      md:ml-64 lg:mr-64
      ${mobileTab !== "chat" ? "hidden md:flex" : "flex"}
    `}>
      {/* Header (Inchangé) */}
      <div className="py-3 border-b border-white/10 mb-4 flex items-center justify-center">
        <h3 className="text-white font-bold text-lg">
          {selectedChannel ? `# ${selectedChannel.name}` : t.chat_select_channel}
        </h3>
        {selectedServer && <span className="text-blue-gray text-xs bg-dark-navy px-2 py-1 rounded">{selectedServer.name}</span>}
      </div>

      {/* Historique */}
      <div ref={messagesRef} onScroll={handleScroll} className="flex-1 space-y-4 mb-4 overflow-y-auto custom-scrollbar px-2">
        {!selectedChannel ? (
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
              <div className="bg-dark-navy text-white px-4 py-2 rounded-lg border border-white/5 break-words relative pr-16 pb-2">
                
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
                ) : isGifUrl(msg.content) ? (
                    <img
                        src={msg.content.trim()}
                        alt="GIF"
                        className="max-w-full rounded-lg max-h-48 object-contain mt-1"
                        loading="lazy"
                    />
                ) : (
                    msg.content
                )}


                {/* --- BOUTON OPTIONS (Caché si mode édition) --- */}
                {editingMessageId !== msg.id && (
                    <button
                        onClick={(e) => {
                            e.preventDefault(); e.nativeEvent.stopImmediatePropagation(); e.stopPropagation();
                            setOpenMenuId(prev => prev === msg.id ? null : msg.id);
                            setReactionPickerId(null);
                        }}
                        className={`absolute top-2 right-2 p-1 rounded hover:bg-white/10 text-gray-400 hover:text-white transition ${openMenuId === msg.id ? "opacity-100 bg-white/10" : "opacity-0 group-hover:opacity-100"}`}
                    >
                        ⋮
                    </button>
                )}

                {/* --- BOUTON RÉACTION (Caché si mode édition) --- */}
                {editingMessageId !== msg.id && (
                    <button
                        onClick={(e) => {
                            e.preventDefault(); e.nativeEvent.stopImmediatePropagation(); e.stopPropagation();
                            setReactionPickerId(prev => prev === msg.id ? null : msg.id);
                            setOpenMenuId(null);
                        }}
                        className={`absolute top-2 right-8 p-1 rounded hover:bg-white/10 text-gray-400 hover:text-white transition text-xs ${reactionPickerId === msg.id ? "opacity-100 bg-white/10" : "opacity-0 group-hover:opacity-100"}`}
                    >
                        😊
                    </button>
                )}

                {/* --- PICKER DE RÉACTIONS --- */}
                {reactionPickerId === msg.id && (
                    <div
                        onClick={(e) => { e.nativeEvent.stopImmediatePropagation(); e.stopPropagation(); }}
                        className="absolute bottom-10 right-0 bg-[#0F0F1A] border border-gray-700 rounded-full shadow-xl z-50 flex items-center gap-1 px-2 py-1"
                    >
                        {REACTION_EMOJIS.map(emoji => (
                            <button
                                key={emoji}
                                onClick={() => toggleReaction(msg.id, emoji)}
                                className={`text-lg hover:scale-125 transition-transform rounded-full w-8 h-8 flex items-center justify-center hover:bg-white/10 ${
                                    msg.reactions.find(r => r.emoji === emoji)?.userIds.includes(user?.id ?? "") ? "bg-white/20" : ""
                                }`}
                            >
                                {emoji}
                            </button>
                        ))}
                    </div>
                )}

                {/* --- RÉACTIONS AFFICHÉES --- */}
                {msg.reactions.length > 0 && (
                    <div className="flex flex-wrap gap-1 mt-2">
                        {msg.reactions.map(r => (
                            <button
                                key={r.emoji}
                                onClick={(e) => { e.nativeEvent.stopImmediatePropagation(); e.stopPropagation(); toggleReaction(msg.id, r.emoji); }}
                                className={`flex items-center gap-1 text-xs px-2 py-0.5 rounded-full border transition hover:scale-105 ${
                                    r.userIds.includes(user?.id ?? "")
                                        ? "border-blue-500 bg-blue-500/20 text-white"
                                        : "border-gray-700 bg-white/5 text-gray-300 hover:border-gray-500"
                                }`}
                            >
                                <span>{r.emoji}</span>
                                <span className="font-semibold">{r.userIds.length}</span>
                            </button>
                        ))}
                    </div>
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
            disabled={!selectedChannel}
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
          placeholder={selectedChannel ? `${t.chat_send_placeholder} # ${selectedChannel.name}` : "..."}
          disabled={!selectedChannel}
          className="flex-1 min-w-0 bg-dark-navy text-white px-3 py-2 rounded-lg outline-none border border-white/10 focus:border-cyan disabled:opacity-30 text-sm"
        />
        <button
          onClick={sendMessage}
          disabled={!selectedChannel}
          className="bg-cyan hover:bg-blue-mid text-white px-3 sm:px-5 py-2 rounded-lg font-bold disabled:opacity-30 transition shadow-lg flex-shrink-0 flex items-center gap-1 text-sm"
        >
          <svg className="w-4 h-4 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
          </svg>
          <span className="hidden sm:inline">{t.chat_send_btn}</span>
        </button>

        {/* Bouton GIF + Picker */}
        <div className="relative">
          <button
            ref={gifBtnRef}
            onClick={() => {
              if (gifBtnRef.current) {
                const rect = gifBtnRef.current.getBoundingClientRect();
                setGifPickerPos({
                  bottom: window.innerHeight - rect.top + 8,
                  right: window.innerWidth - rect.right,
                });
              }
              setShowGifPicker(p => !p);
              setShowEmojiPicker(false);
            }}
            disabled={!selectedChannel}
            className="p-2 rounded-lg bg-dark-navy hover:bg-navy-deep text-white text-xs font-bold disabled:opacity-30 transition border border-white/10 h-10 px-3"
          >
            GIF
          </button>
        </div>

        {/* GIF Picker — fixed pour éviter tout problème d'overflow parent */}
        {showGifPicker && (
          <div
            ref={gifPickerRef}
            className="fixed bg-[#0F0F1A] border border-gray-700 rounded-xl shadow-2xl z-[9999] flex flex-col overflow-hidden"
            style={{
              bottom: gifPickerPos.bottom,
              right: gifPickerPos.right,
              width: "min(380px, 92vw)",
            }}
          >
            {/* Barre de recherche */}
            <div className="p-3 border-b border-gray-700 flex items-center gap-2">
              <span className="text-lg">🔍</span>
              <input
                autoFocus
                type="text"
                placeholder="Rechercher un GIF..."
                value={gifSearch}
                onChange={e => setGifSearch(e.target.value)}
                className="flex-1 bg-black/40 border border-gray-600 rounded-lg px-3 py-1.5 text-sm text-white outline-none focus:border-cyan placeholder-gray-500"
              />
              <button
                onClick={() => setShowGifPicker(false)}
                className="text-gray-400 hover:text-white transition p-1 text-lg leading-none"
              >
                ✕
              </button>
            </div>

            {/* Label tendances / résultats */}
            <div className="px-3 pt-2 pb-1 text-xs text-gray-400 font-semibold uppercase tracking-wider">
              {gifSearch.trim() ? `Résultats pour "${gifSearch}"` : "Tendances"}
            </div>

            {/* Grille de GIFs */}
            <div className="overflow-y-auto" style={{ maxHeight: "260px" }}>
              {gifLoading ? (
                <div className="flex items-center justify-center py-10 text-gray-400 text-sm gap-2">
                  <span className="animate-spin">⏳</span> Chargement...
                </div>
              ) : gifResults.length === 0 ? (
                <div className="text-center py-10 text-gray-500 text-sm">Aucun résultat</div>
              ) : (
                <div className="grid grid-cols-3 gap-1 p-2">
                  {gifResults.map(gif => (
                    <button
                      key={gif.id}
                      onClick={() => sendGif(gif.url)}
                      className="rounded-md overflow-hidden hover:ring-2 hover:ring-blue-400 transition-all"
                    >
                      <img
                        src={gif.preview}
                        alt="gif"
                        className="w-full h-20 object-cover"
                        loading="lazy"
                      />
                    </button>
                  ))}
                </div>
              )}
            </div>

            {/* Footer Giphy */}
            <div className="px-3 py-2 border-t border-gray-700 flex items-center justify-end">
              <span className="text-[10px] text-gray-500">Powered by GIPHY</span>
            </div>
          </div>
        )}

      </div>
    </div>
  );
}
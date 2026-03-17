"use client";

import { createContext, useContext, useState, useEffect, useRef, ReactNode } from "react";
import { Server } from "../components/chatbar";
import { triggerSystemNotification } from "./utils/notifications"; // Adapter le chemin relatif si besoin
import { useLang } from "./langContext";

export interface User {
  id: string;
  username: string;
  display_name?: string;
  avatar_url?: string;
  serverList?: Server[];
}

interface AuthState {
  user: User | null;
  servers: Server[];
  isLoading: boolean;
  socket: WebSocket | null;
  banNotification: { message: string; serverId: number } | null;
  clearBanNotification: () => void;
  addServer: (server: Server) => void;
  setServers: (servers: Server[]) => void; 
  logout: () => void;
  refreshUserData: () => Promise<void>; // Utile pour recharger sans F5
  connectWs: () => void;
}

const AuthContext = createContext<AuthState | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [servers, setServers] = useState<Server[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [banNotification, setBanNotification] = useState<{ message: string; serverId: number } | null>(null);

  const { t } = useLang();
  const tRef = useRef(t);
  useEffect(() => { tRef.current = t; }, [t]);

  const wsRef = useRef<WebSocket | null>(null);

  // Ref pour accéder à l'état user courant dans le callback WS sans le recréer
  const userRef = { current: user };
  const clearBanNotification = () => setBanNotification(null);
  // Ref stable pour lire le user_id courant depuis la closure WS (stale closure fix)
  const currentUserIdRef = useRef<string | null>(null);

  // Fonction unique de récupération de données
  const refreshUserData = async () => {
    const token = localStorage.getItem("access_token");
    if (!token) {
      setIsLoading(false);
      return;
    }

    try {
      const res = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/me`, {
        headers: { Authorization: `Bearer ${token}` },
      });

      if (res.ok) {
        const data = await res.json();

        setUser({
          id: data.id,
          username: data.username,
          display_name: data.display_name,
          avatar_url: data.avatar_url,
        });
        // Stocker l'ID pour la détection des events WS (kick/ban)
        currentUserIdRef.current = String(data.id);
        localStorage.setItem("current_user_id", data.id);

        const mappedServers = (data.servers || []).map((s: any) => ({
          ...s,
          invitcode: s.invitcode ? String(s.invitcode) : "",
        }));
        setServers(mappedServers);

        // Show notifications for bans that happened while offline
      } else {
        localStorage.removeItem("access_token");
      }
    } catch (error) {
      console.error("Auth init failed", error);
    } finally {
      setIsLoading(false);
    }
  };

  const connectWs = () => {
      if (typeof window === "undefined") return;

      if (wsRef.current) {
          wsRef.current.close();
      }

      const wsUrl = `${process.env.NEXT_PUBLIC_WS_URL}/ws`;
      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
          console.log("🟢 WS Connecté");
          const token = localStorage.getItem("access_token");
          if (token) {
              ws.send(JSON.stringify({ type: "auth", token: token }));
          } else {
              // Si aucun token n'est présent, on arrête le chargement (utilisateur déconnecté)
              setIsLoading(false);
          }
      };

      ws.onclose = () => console.log("🔴 WS Déconnecté");
      ws.onerror = (err) => console.error("⚠️ WS Erreur", err);

      ws.onmessage = async (event) => {
          try {
              const parsed = JSON.parse(event.data);
              
              if (parsed.type === "auth_success") {
                  console.log("🔒 Auth WS réussie, chargement des données...");
                  await refreshUserData(); 
              }

              if (!parsed || !parsed.data) return;
              const data = parsed.data;
              // --- GESTION DU STATUT EN TEMPS RÉEL (Vert/Gris) ---
              if (parsed.type === "user_status_change") {
                  setServers((prev: any[]) => prev.map(server => {
                      // Utiliser String() assure que 1 (number) == "1" (string)
                      if (String(server.id) === String(data.server_id)) {
                          return {
                              ...server,
                              members: (server.members || []).map((m: any) => {
                                  // Comparaison sécurisée user_id
                                  if (String(m.user_id) === String(data.user_id)) {
                                      return { ...m, status: data.status }; 
                                  }
                                  return m;
                              })
                          };
                      }
                      return server;
                  }));
              }
              // ---------------------------------------------------

              if (parsed.type === "user_kicked") {
                  // Supprimer le membre de la liste ET notifier si c'est moi
                  const currentUserId = localStorage.getItem("current_user_id");
                  if (currentUserId && String(data.user_id) === String(currentUserId)) {
                        setBanNotification({ message: t.ban_offline_kicked(data.server_name), serverId: data.server_id });
                        setServers((prev: any[]) => prev.filter((s: any) => s.id !== data.server_id));
                        triggerSystemNotification(t.ban_label_kick, t.ban_offline_kicked(data.server_name));
                  } else {
                      setServers((prev: any[]) => prev.map((server: any) => {
                          if (server.id === data.server_id) {
                              return { ...server, members: (server.members || []).filter((m: any) => String(m.id) !== String(data.member_id)) };
                          }
                          return server;
                      }));
                  }
              }

              if (parsed.type === "user_banned") {
                  const currentUserId = localStorage.getItem("current_user_id");
                  if (currentUserId && String(data.user_id) === String(currentUserId)) {
                      let msg: string;
                      if (!data.banned_until) {
                          msg = t.ban_permanent;
                      } else {
                          // Calculer la différence pour détecter un kick (10s)
                          const until = new Date(data.banned_until);
                          const diffSecs = Math.round((until.getTime() - Date.now()) / 1000);
                          if (diffSecs <= 15) {
                              msg = t.ban_offline_kicked(data.server_name);
                          } else {
                              const hours = Math.floor(diffSecs / 3600);
                              const days = Math.floor(diffSecs / 86400);
                              if (days >= 1) msg = t.ban_offline_days(data.server_name, days);
                              else if (hours >= 1) msg = t.ban_offline_hours(data.server_name, hours);
                              else msg = t.ban_offline_minutes(data.server_name, Math.ceil(diffSecs / 60));
                          }
                      }
                        setBanNotification({ message: msg, serverId: data.server_id });
                        triggerSystemNotification(t.ban_label, msg);
                        setServers((prev: any[]) => prev.filter((s: any) => s.id !== data.server_id));
                  } else {
                      // Retirer le membre banni de la liste pour les autres
                      setServers((prev: any[]) => prev.map((server: any) => {
                          if (server.id === data.server_id) {
                              return { ...server, members: (server.members || []).filter((m: any) => String(m.id) !== String(data.member_id)) };
                          }
                          return server;
                      }));
                  }
              }

              if (parsed.type === "user_joined") {
                  setServers((prevServers: any[]) => {
                      return prevServers.map((server) => {
                          if (server.id === data.server_id) {
                              const currentMembers = server.members || [];
                              // Fix: Comparer user_id pour éviter doublons sur reconnexion
                              const exists = currentMembers.some((m: any) => String(m.user_id) === String(data.member.user_id));
                              
                              if (!exists) {
                                  return {
                                      ...server,
                                      members: [...currentMembers, data.member]
                                  };
                              }
                          }
                          return server;
                      });
                  });
              }
              
              // MODIFICATION ICI : On recharge tout pour gérer les changements de rôles (Admins/Owner)
              if (parsed.type === "member_updated") {
                  await refreshUserData();
              }

          } catch (error) {
              console.error("Erreur parsing WS context", error);
          }
      };

      setSocket(ws);
  };

  useEffect(() => {
    connectWs();
    return () => {
        if (wsRef.current) wsRef.current.close();
    };
  }, []); 

  const logout = async () => {
    const token = localStorage.getItem("access_token");
    
    // 1. Notifier le backend (qui notifiera les autres via WebSocket)
    if (token) {
        try {
            // Adaptez l'URL si votre route est différente (/auth/logout ou /logout)
            await fetch(`${process.env.NEXT_PUBLIC_API_URL}/auth/logout`, {
                method: "POST",
                headers: { 
                    "Authorization": `Bearer ${token}` 
                    // Pas besoin de Content-Type si pas de body
                },
            });
        } catch (err) {
            console.error("Erreur appel API logout", err);
        }
    }

    // 2. Nettoyage Frontend
    if (socket) {
        socket.close();
        setSocket(null);
    }
    localStorage.removeItem("access_token");
    localStorage.removeItem("username");
    localStorage.removeItem("current_user_id");
    setUser(null);
    setServers([]);
    
    // 3. Redirection
    window.location.href = "http://localhost:3001";
  };

  const addServer = (server: Server) => {
    setServers((prev) => [...prev, server]);
  };

  return (
    <AuthContext.Provider value={{ user, servers, isLoading, socket, banNotification, clearBanNotification, setServers, addServer, logout, refreshUserData, connectWs }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error("useAuth must be used within an AuthProvider");
  }
  return context;
}
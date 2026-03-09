"use client";

import { createContext, useContext, useState, useEffect, ReactNode } from "react";
import { Server } from "../components/chatbar"; // Assurez-vous que le chemin est bon

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
  addServer: (server: Server) => void;
  setServers: (servers: Server[]) => void; 
  logout: () => void;
  refreshUserData: () => Promise<void>; // Utile pour recharger sans F5
}

const AuthContext = createContext<AuthState | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [servers, setServers] = useState<Server[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [socket, setSocket] = useState<WebSocket | null>(null);

  // Fonction unique de récupération de données
  const refreshUserData = async () => {
    const token = localStorage.getItem("access_token");
    if (!token) {
      setIsLoading(false);
      return;
    }

    try {
      const res = await fetch("http://localhost:3000/me", {
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

        const mappedServers = (data.servers || []).map((s: any) => ({
          ...s,
          invitcode: s.invitcode ? String(s.invitcode) : "",
        }));
        setServers(mappedServers);
      } else {
        localStorage.removeItem("access_token");
      }
    } catch (error) {
      console.error("Auth init failed", error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    // Eviter de créer un socket si on n'est pas sur le client
    if (typeof window === "undefined") return;

    // FIX 1: Déclarer ws ici pour qu'il soit accessible au cleanup ET à connectWs
    let ws: WebSocket | null = null;

    const connectWs = () => {
        const wsUrl = "ws://localhost:3000/ws";
        
        // FIX 2: Ne pas utiliser 'const', on assigne la variable du scope parent
        ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            console.log("🟢 WS Connecté");
            const token = localStorage.getItem("access_token");
            if (token) {
                ws?.send(JSON.stringify({ type: "auth", token: token }));
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
                    console.log("⚡ Update Statut Reçu:", data); // Log de debug
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
                    console.log("🔄 Update Member reçue, rafraîchissement des permissions...");
                    await refreshUserData();
                }

            } catch (error) {
                console.error("Erreur parsing WS context", error);
            }
        };

        setSocket(ws);
    };

    connectWs();

    // Cleanup: ws est maintenant accessible ici grâce au FIX 1
    return () => {
        if (ws) ws.close();
    };
  }, []); 

  const logout = async () => {
    const token = localStorage.getItem("access_token");
    
    // 1. Notifier le backend (qui notifiera les autres via WebSocket)
    if (token) {
        try {
            // Adaptez l'URL si votre route est différente (/auth/logout ou /logout)
            await fetch("http://localhost:3000/auth/logout", {
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
    setUser(null);
    setServers([]);
    
    // 3. Redirection
    window.location.href = "http://localhost:3001";
  };

  const addServer = (server: Server) => {
    setServers((prev) => [...prev, server]);
  };

  return (
    <AuthContext.Provider value={{ user, servers, isLoading, socket, setServers, addServer, logout, refreshUserData }}>
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
"use client";

import React, { useState, useMemo } from "react";
import Image from "next/image";
import { useAuth } from "../app/context";

// Types
interface MemberItem {
  id: string; 
  user_id: string;
  username: string;
  display_name?: string;
  avatar_url?: string;
  role: string;
  status?: string; // Ajout status
}

interface ServerWithMembers {
  id: number;
  name: string;
  owner_id: string;
}

interface Props {
  selectedServer?: ServerWithMembers | null;
}

export default function MembersBar({ selectedServer }: Props) {
  // On récupère 'servers' pour toujours avoir les données à jour (fix du problème d'affichage)
  const { user, servers, refreshUserData } = useAuth(); // AJOUT DE refreshUserData
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);

  const roleColors: Record<string, string> = {
    Owner: "text-red-500",
    Admin: "text-blue-500",
    Member: "text-gray-400",
  };

  const statusColors: Record<string, string> = {
    // Minuscules (cas par défaut)
    online: "#22c55e", 
    Online: "#22c55e",
    // Majuscules (venant de la DB Rust Enum)
    offline: "#6b7280",
    Offline: "#6b7280",
  };

  // 1. Récupérer le "vrai" serveur depuis le contexte pour éviter d'avoir des données obsolètes
  const activeServer = useMemo(() => {
    if (!selectedServer) return null;
    // On cherche dans la liste globale le serveur qui a le même ID
    // (cast 'any' temporaire si les types Server du context et d'ici diffèrent légèrement sur 'members')
    return servers.find(s => s.id === selectedServer.id) as any; 
  }, [selectedServer, servers]);

  // 2. Traitement des membres
  const { filteredMembers, currentUserRole } = useMemo(() => {
    if (!activeServer || !activeServer.members) {
      return { filteredMembers: [], currentUserRole: "Member" };
    }

    const rawMembers: MemberItem[] = activeServer.members;

    // Transformation
    let members = rawMembers.map((m) => ({
      ...m,
      role: m.role || "Member",
      displayName: m.display_name || m.username,
      status: m.status || "Offline", // UTILISE le vrai status, default Offline
      user_id: String(m.user_id) 
    }));

    // Trouver mon rôle (si je suis connecté)
    let myRole = "Member";
    if (user) {
        const me = members.find((m) => m.user_id === String(user.id));
        if (me) myRole = me.role;
    }

    // Tri
    const roleOrder: Record<string, number> = { Owner: 0, Admin: 1, Member: 2 };
    members.sort((a, b) => {
      const roleDiff = (roleOrder[a.role] ?? 2) - (roleOrder[b.role] ?? 2);
      if (roleDiff !== 0) return roleDiff;
      return a.displayName.localeCompare(b.displayName);
    });
    
    return { filteredMembers: members, currentUserRole: myRole };
}, [activeServer, user]);

  const handleAction = async (action: string, targetUserId: string) => {
    setOpenMenuId(null);
    if (!activeServer) return;
    const token = localStorage.getItem("access_token");

    try {
        let newRole = "";

        if (action === "promote") newRole = "Admin";
        else if (action === "demote") newRole = "Member";
        else if (action === "transfer") {
            // Sécurité côté client pour une action critique
            if (!window.confirm(`Êtes-vous sûr de vouloir transférer la propriété du serveur ? Vous deviendrez Admin.`)) {
                return;
            }
            newRole = "Owner";
        }

        if (newRole) {
            const res = await fetch(`http://localhost:3000/servers/${activeServer.id}/members/${targetUserId}`, {
                method: "PUT",
                headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
                body: JSON.stringify({ new_role: newRole })
            });

            if (res.ok) {
                await refreshUserData(); // Mise à jour immédiate pour l'admin qui clique
            }
        }
    } catch (e) {
        console.error(e);
    }
};


  return (
    <div className="fixed top-16 right-0 h-[calc(100vh-4rem)] w-64 bg-[#001839] border-l border-[#3D3D3D] flex flex-col p-4 z-10 shadow-lg">
      <h2 className="text-gray-300 text-xs font-bold uppercase mb-4 flex items-center gap-2 tracking-wider">
        Membres — {filteredMembers.length}
      </h2>

      <div className="flex-1 overflow-y-auto space-y-2 pr-1 scrollbar-thin scrollbar-thumb-gray-700">
        {!activeServer ? (
          <div className="text-gray-500 text-center text-sm italic mt-10">Sélectionnez un serveur</div>
        ) : (
            filteredMembers.map((member) => {
                const statusHex = statusColors[member.status || "Offline"] || "#6b7280";
                const isMe = user ? String(user.id) === member.user_id : false;
                
                const canManage = user && !isMe && (
                    (currentUserRole === "Owner") || 
                    (currentUserRole === "Admin" && member.role === "Member")
                );
                
            const statusClass = statusColors[member.status || "Offline"] || "bg-gray-500";
            return (
              <div 
                key={member.id} 
                className="group relative flex items-center gap-3 p-2 rounded hover:bg-[#1E1E2E] transition-colors cursor-pointer"
                onMouseLeave={() => setOpenMenuId(null)}
              >
                <div className="relative flex-shrink-0">
                  <Image 
                    src={"/images/user.png"} 
                    alt={member.username} 
                    width={32} height={32} 
                    className="rounded-full bg-gray-700" 
                  />
                <div 
                    className="absolute bottom-0 right-0 w-3 h-3 rounded-full border-2 border-[#11111b]"
                    style={{ backgroundColor: statusHex }}
                  />
                </div>
                <div className="flex-1 min-w-0">
                  <div className={`text-sm font-medium truncate ${isMe ? "text-blue-400 font-bold" : "text-gray-300"}`}>
                    {member.displayName} {isMe && "(Moi)"}
                  </div>
                  {member.role !== "Member" && (
                     <div className={`text-[10px] font-bold uppercase mt-0.5 flex gap-1 ${roleColors[member.role]}`}>
                        {member.role === "Owner" && "👑"} {member.role}
                     </div>
                  )}
                </div>

                {canManage && (
                    <button
                        onClick={(e) => { 
                            e.stopPropagation(); 
                            // Correction ici pour la cohérence de l'ID d'ouverture
                            setOpenMenuId(openMenuId === member.id ? null : member.id); 
                        }}
                        className="opacity-0 group-hover:opacity-100 text-gray-400 hover:text-white transition px-1"
                    >
                        ⋮
                    </button>
                )}

                {openMenuId === member.id && (
                    <div className="absolute right-8 top-8 w-48 bg-[#181825] rounded shadow-xl border border-[#3D3D3D] z-50 overflow-hidden text-sm animate-in fade-in zoom-in-95 duration-100">
                        
                        {/* Gestion Rôle Admin/Membre */}
                        {currentUserRole === "Owner" && member.role === "Member" && (
                            <button onClick={() => handleAction("promote", member.user_id)} className="w-full text-left px-3 py-2 text-gray-300 hover:bg-blue-600 hover:text-white transition flex gap-2">
                                🛡️ Promouvoir Admin
                            </button>
                        )}
                        
                        {currentUserRole === "Owner" && member.role === "Admin" && (
                             <button onClick={() => handleAction("demote", member.user_id)} className="w-full text-left px-3 py-2 text-gray-300 hover:bg-yellow-600 hover:text-white transition flex gap-2">
                                ⬇️ Rétrograder
                            </button>
                        )}

                        {/* Transfert de Propriété (Uniquement Owner) */}
                        {currentUserRole === "Owner" && (
                             <button onClick={() => handleAction("transfer", member.user_id)} className="w-full text-left px-3 py-2 text-amber-500 hover:bg-amber-600 hover:text-white transition flex gap-2 border-t border-[#3D3D3D]">
                                👑 Transférer propriété
                            </button>
                        )}

                        {/* Kick (Toujours dispo si canManage est true, sauf si Owner vs Admin géré par canManage) */}
                        <button className="w-full text-left px-3 py-2 text-red-400 hover:bg-red-600 hover:text-white border-t border-[#3D3D3D] transition flex gap-2">
                            🚪 Expulser
                        </button>
                    </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
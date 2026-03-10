"use client";

import React, { useState, useMemo } from "react";
import Image from "next/image";
import { useAuth } from "../app/context";
import { useLang } from "../app/langContext";

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
  userStatus?: string;
  selectedChannel?: any;
  mobileTab?: string;
}

export default function MembersBar({ selectedServer, userStatus, mobileTab }: Props) {
  const { user, servers, refreshUserData } = useAuth();
  const { t } = useLang();
  const [openMenuId, setOpenMenuId] = useState<string | null>(null);
  const [banModal, setBanModal] = useState<{ userId: string; displayName: string } | null>(null);
  const [banValue, setBanValue] = useState("30");
  const [banUnit, setBanUnit] = useState<"minutes" | "heures" | "jours">("minutes");
  const [banError, setBanError] = useState("");
  const [actionError, setActionError] = useState("");

  const roleColors: Record<string, string> = {
    Owner: "text-red-500",
    Admin: "text-blue-500",
    Member: "text-gray-400",
  };

  const statusColor = (status?: string) => {
    switch ((status || "").toLowerCase()) {
      case "online":    return "#22c55e"; // vert
      case "offline":   return "#ef4444"; // rouge
      case "invisible": return "#6b7280"; // gris (paraît hors-ligne pour les autres)
      default: return "#6b7280";
    }
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
            if (!window.confirm(t.members_transfer_confirm)) {
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
                await refreshUserData();
            }
            return;
        }

        if (action === "kick") {
            if (!window.confirm(t.members_kick_confirm)) return;
            const res = await fetch(`http://localhost:3000/servers/${activeServer.id}/ban/${targetUserId}`, {
                method: "POST",
                headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
                body: JSON.stringify({ duration: 10 })
            });
            if (!res.ok) {
                const err = await res.json().catch(() => ({}));
                setActionError(err.error || t.members_kick_error);
            }
            return;
        }

        if (action === "permban") {
            if (!window.confirm(t.members_perm_ban_confirm)) return;
            const res = await fetch(`http://localhost:3000/servers/${activeServer.id}/ban/${targetUserId}`, {
                method: "POST",
                headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
                body: JSON.stringify({ duration: null })
            });
            if (!res.ok) {
                const err = await res.json().catch(() => ({}));
                setActionError(err.error || t.members_perm_ban_error);
            }
            return;
        }
    } catch (e) {
        console.error(e);
        setActionError(t.members_network_error);
    }
};

  const handleConfirmTempBan = async () => {
    if (!banModal || !activeServer) return;
    setBanError("");
    const val = parseInt(banValue, 10);
    if (isNaN(val) || val <= 0) {
        setBanError(t.members_ban_invalid_value);
        return;
    }
    const multipliers: Record<"minutes" | "heures" | "jours", number> = { minutes: 60, heures: 3600, jours: 86400 };
    const durationSeconds = val * multipliers[banUnit as "minutes" | "heures" | "jours"];
    const token = localStorage.getItem("access_token");
    try {
        const res = await fetch(`http://localhost:3000/servers/${activeServer.id}/ban/${banModal.userId}`, {
            method: "POST",
            headers: { "Content-Type": "application/json", "Authorization": `Bearer ${token}` },
            body: JSON.stringify({ duration: durationSeconds })
        });
        if (!res.ok) {
            const err = await res.json().catch(() => ({}));
            setBanError(err.error || t.members_temp_ban_error);
        } else {
            setBanModal(null);
            setBanValue("30");
            setBanUnit("minutes");
        }
    } catch (e) {
        setBanError(t.members_network_error);
    }
  };


  return (
    <>
      <div className={`
        fixed right-0 bg-[#001839] border-l border-[#3D3D3D] flex-col p-4 z-10 shadow-lg
        top-16 w-full h-[calc(100vh-8rem)]
        md:w-64 md:h-[calc(100vh-4rem)]
        ${mobileTab === "members" ? "flex" : "hidden"}
        md:flex
      `}>

      {/* Modal ban temporaire */}
      {banModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onClick={() => setBanModal(null)}>
          <div className="bg-[#181825] border border-[#3D3D3D] rounded-xl p-6 w-80 shadow-2xl" onClick={e => e.stopPropagation()}>
            <h3 className="text-white font-bold text-base mb-1">{t.members_ban_modal_title}</h3>
            <p className="text-gray-400 text-sm mb-4">{t.members_ban_for} <span className="text-orange-400 font-semibold">{banModal.displayName}</span></p>
            <div className="flex gap-2 mb-3">
              <input
                type="number"
                min="1"
                value={banValue}
                onChange={e => setBanValue(e.target.value)}
                className="w-20 bg-[#11111b] border border-[#3D3D3D] rounded px-2 py-1.5 text-white text-sm focus:outline-none focus:border-orange-500"
              />
              <select
                value={banUnit}
                onChange={e => setBanUnit(e.target.value as any)}
                className="flex-1 bg-[#11111b] border border-[#3D3D3D] rounded px-2 py-1.5 text-white text-sm focus:outline-none focus:border-orange-500"
              >
                <option value="minutes">{t.members_ban_minutes}</option>
                <option value="heures">{t.members_ban_hours}</option>
                <option value="jours">{t.members_ban_days}</option>
              </select>
            </div>
            {banError && <p className="text-red-400 text-xs mb-3">{banError}</p>}
            <div className="flex gap-2 mt-2">
              <button onClick={() => setBanModal(null)} className="flex-1 px-3 py-2 rounded bg-[#11111b] text-gray-400 hover:text-white text-sm transition">{t.members_cancel}</button>
              <button onClick={handleConfirmTempBan} className="flex-1 px-3 py-2 rounded bg-orange-600 hover:bg-orange-500 text-white font-semibold text-sm transition">{t.members_confirm}</button>
            </div>
          </div>
        </div>
      )}

      {/* Toast erreur action */}
      {actionError && (
        <div className="absolute top-2 left-2 right-2 bg-red-700 text-white text-xs rounded p-2 z-40 flex justify-between items-center">
          <span>{actionError}</span>
          <button onClick={() => setActionError("")} className="ml-2 font-bold">✕</button>
        </div>
      )}
      <h2 className="text-gray-300 text-xs font-bold uppercase mb-4 flex items-center gap-2 tracking-wider">
        {t.members_title} — {filteredMembers.length}
      </h2>

      <div className="flex-1 overflow-y-auto space-y-2 pr-1 scrollbar-thin scrollbar-thumb-gray-700">
        {!activeServer ? (
          <div className="text-gray-500 text-center text-sm italic mt-10">{t.members_select_server}</div>
        ) : (
            filteredMembers.map((member) => {
                const isMe = user ? String(user.id) === member.user_id : false;
                const statusHex = isMe && userStatus ? statusColor(userStatus) : statusColor(member.status);
                
                const canManage = user && !isMe && (
                    (currentUserRole === "Owner") || 
                    (currentUserRole === "Admin" && member.role === "Member")
                );
                
            return (
              <div 
                key={member.id} 
                className="group relative flex items-center gap-3 p-2 rounded hover:bg-[#1E1E2E] transition-colors cursor-pointer"
                onMouseLeave={() => setOpenMenuId(null)}
              >
                <div className="relative flex-shrink-0">
                  <Image 
                    src={member.avatar_url || user?.avatar_url || "/default-avatar.png"} 
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
                    {member.displayName} {isMe && `(${t.members_me})`}
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
                                {t.members_promote}
                            </button>
                        )}
                        
                        {currentUserRole === "Owner" && member.role === "Admin" && (
                             <button onClick={() => handleAction("demote", member.user_id)} className="w-full text-left px-3 py-2 text-gray-300 hover:bg-yellow-600 hover:text-white transition flex gap-2">
                                {t.members_demote}
                            </button>
                        )}

                        {currentUserRole === "Owner" && (
                             <button onClick={() => handleAction("transfer", member.user_id)} className="w-full text-left px-3 py-2 text-amber-500 hover:bg-amber-600 hover:text-white transition flex gap-2 border-t border-[#3D3D3D]">
                                {t.members_transfer}
                            </button>
                        )}

                        <button onClick={() => handleAction("kick", member.user_id)} className="w-full text-left px-3 py-2 text-red-400 hover:bg-red-600 hover:text-white border-t border-[#3D3D3D] transition flex gap-2">
                            {t.members_kick}
                        </button>

                        <button onClick={() => { setOpenMenuId(null); setBanModal({ userId: member.user_id, displayName: member.displayName }); }} className="w-full text-left px-3 py-2 text-orange-400 hover:bg-orange-600 hover:text-white border-t border-[#3D3D3D] transition flex gap-2">
                            {t.members_temp_ban}
                        </button>

                        <button onClick={() => handleAction("permban", member.user_id)} className="w-full text-left px-3 py-2 text-red-500 hover:bg-red-700 hover:text-white border-t border-[#3D3D3D] transition flex gap-2 font-semibold">
                            {t.members_perm_ban}
                        </button>
                        
                    </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
    </>
  );
}
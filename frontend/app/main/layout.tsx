"use client";

import Navbar from "../../components/navbar";
import Mainbar from "../../components/mainbar";
import Userbar from "../../components/userbar";
import ChatBar, { Server, Channel } from "../../components/chatbar";
import MembersBar from "../../components/membersbar";
import Chat from "../../components/chat";
import { useEffect, useState } from "react";
import { useAuth } from "../context";

type UserStatus = "online" | "offline" | "invisible";
export type MobileTab = "channels" | "chat" | "members" | "profile";

export default function MainLayout({ children }: { children: React.ReactNode }) {
  const [username, setUsername] = useState<string>("");
  const [userStatus, setUserStatus] = useState<UserStatus>("online");
  const { banNotification, clearBanNotification, setServers } = useAuth();

  const [selectedServer, setSelectedServer] = useState<Server | null>(null);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);
  const [mobileTab, setMobileTab] = useState<MobileTab>("channels");

  // NOUVEAUX ÉTATS POUR GÉRER L'ONGLET ET LE DM
  const [activeTab, setActiveTab] = useState<"servers" | "dms">("servers");
  const [activeDMUser, setActiveDMUser] = useState<{ id: string; name: string } | null>(null);
  const [showMembersOnTablet, setShowMembersOnTablet] = useState(false);


  // Si on est banni/expulsé du serveur actuellement sélectionné, le désélectionner
  useEffect(() => {
    if (banNotification && selectedServer && selectedServer.id === banNotification.serverId) {
      setSelectedServer(null);
      setSelectedChannel(null);
    }
  }, [banNotification]);

  useEffect(() => {
    const storedUsername = localStorage.getItem("username");
    if (storedUsername) setUsername(storedUsername);
  }, []);

  // NOUVEAU : Ecoute le changement d'onglet pour nettoyer automatiquement le tchat affiché
  useEffect(() => {
    if (activeTab === "servers") {
      setActiveDMUser(null);
    } else if (activeTab === "dms") {
      setSelectedServer(null);
      setSelectedChannel(null);
    }
  }, [activeTab]);

  const handleServerSelect = (server: Server | null) => { // <-- Ajout de | null
    setSelectedServer(server);
    setSelectedChannel(null); // On reset le salon pour forcer l'utilisateur à en choisir un
    setActiveDMUser(null);    // On quitte le mode DM
    
    // On rebascule sur l'onglet serveur UNIQUEMENT si on a cliqué sur un vrai serveur 
    // (et non lors du nettoyage de changement d'onglet)
    if (server) {
        setActiveTab("servers");  
    }
  };

  const handleChannelSelect = (channel: Channel | null) => { // <-- Ajout de | null
    setSelectedChannel(channel);
    setActiveDMUser(null);    // On quitte le mode DM
    
    // Si channel est null, on s'arrête là (ce qui arrive quand on change d'onglet)
    if (!channel) return;

    if (channel.server_id && (!selectedServer || selectedServer.id !== channel.server_id)) {
      setSelectedServer({ id: channel.server_id } as Server);
    }
    // Auto-switch to chat when a channel is picked on mobile
    setMobileTab("chat");
  };

  // NOUVELLE ACTION : Démarrer un message privé depuis MembersBar
  const handleStartDM = (userId: string, username: string) => {
    setSelectedServer(null);
    setSelectedChannel(null);
    setActiveTab("dms");
    setActiveDMUser({ id: userId, name: username });
  };

  return (
    <>
      <Navbar selectedServer={selectedServer} />
      <Mainbar />

      {/* Notification ban/expulsion */}
      {banNotification && (
        <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/70">
          <div className="bg-[#181825] border border-red-600 rounded-2xl p-8 max-w-sm w-full mx-4 shadow-2xl text-center">
            <div className="text-4xl mb-4">
              {banNotification.message.startsWith("❌") ? "🔨" : banNotification.message.startsWith("⏳") ? "⏳" : "🚪"}
            </div>
            <p className="text-white font-semibold text-lg mb-2">Action du serveur</p>
            <p className="text-gray-300 text-sm mb-6">{banNotification.message}</p>
            <button
              onClick={clearBanNotification}
              className="px-6 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg font-bold transition"
            >
              OK
            </button>
          </div>
        </div>
      )}

      {/* Bouton flottant membersbar (visible en md-lg seulement) */}
      {activeTab === "servers" && selectedServer && (
        <button
          onClick={() => setShowMembersOnTablet(!showMembersOnTablet)}
          className="fixed bottom-20 right-4 z-40 hidden md:flex lg:flex xl:hidden bg-blue-600 hover:bg-blue-700 text-white p-3 rounded-full shadow-lg transition"
          title={showMembersOnTablet ? "Masquer les membres" : "Afficher les membres"}
        >
          <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      )}
      
      <ChatBar 
        onServerSelect={handleServerSelect} 
        onChannelSelect={handleChannelSelect} 
        activeTab={activeTab} 
        setActiveTab={setActiveTab} 
        onDMSelect={handleStartDM}
        username={username}
        mobileTab={mobileTab}
      />
      
      {/* On n'affiche la barre des membres que sur l'onglet Serveurs */}
      {activeTab === "servers" && (
        <MembersBar
          userStatus={userStatus}
          selectedServer={selectedServer}
          selectedChannel={selectedChannel}
          onStartDM={handleStartDM}
          mobileTab={mobileTab}
          showOnTablet={showMembersOnTablet}
          onTabletClose={() => setShowMembersOnTablet(false)}
        />
      )}

      <Chat
        selectedServer={selectedServer}
        selectedChannel={selectedChannel}
        activeDMUser={activeDMUser}
        mobileTab={mobileTab}
      />

      <Userbar
        username={username}
        onStatusChange={setUserStatus}
        mobileTab={mobileTab}
      />

      {/* ── Discord-style bottom navigation (mobile only) ── */}
      <nav className="md:hidden fixed bottom-0 left-0 right-0 z-50 bg-[#001839] border-t border-[#3D3D3D] flex h-16">

        {/* Salons */}
        <button
          onClick={() => setMobileTab("channels")}
          className={`flex-1 flex flex-col items-center justify-center gap-0.5 text-[10px] font-semibold transition-colors ${mobileTab === "channels" ? "text-blue-400" : "text-gray-500"}`}
        >
          <svg className={`w-6 h-6 transition-transform ${mobileTab === "channels" ? "scale-110" : ""}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 10h16M4 14h8" />
          </svg>
          Salons
        </button>

        {/* Chat */}
        <button
          onClick={() => setMobileTab("chat")}
          className={`flex-1 flex flex-col items-center justify-center gap-0.5 text-[10px] font-semibold transition-colors ${mobileTab === "chat" ? "text-blue-400" : "text-gray-500"}`}
        >
          <svg className={`w-6 h-6 transition-transform ${mobileTab === "chat" ? "scale-110" : ""}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
          </svg>
          Chat
        </button>

        {/* Membres */}
        <button
          onClick={() => setMobileTab("members")}
          className={`flex-1 flex flex-col items-center justify-center gap-0.5 text-[10px] font-semibold transition-colors ${mobileTab === "members" ? "text-blue-400" : "text-gray-500"}`}
        >
          <svg className={`w-6 h-6 transition-transform ${mobileTab === "members" ? "scale-110" : ""}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
          Membres
        </button>

        {/* Profil */}
        <button
          onClick={() => setMobileTab("profile")}
          className={`flex-1 flex flex-col items-center justify-center gap-0.5 text-[10px] font-semibold transition-colors ${mobileTab === "profile" ? "text-blue-400" : "text-gray-500"}`}
        >
          <svg className={`w-6 h-6 transition-transform ${mobileTab === "profile" ? "scale-110" : ""}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
          {username || "Profil"}
        </button>
      </nav>

      {children}
    </>
  );
}
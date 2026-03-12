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

export default function MainLayout({ children }: { children: React.ReactNode }) {
  const [username, setUsername] = useState<string>("");
  const [userStatus, setUserStatus] = useState<UserStatus>("online");
  const { banNotification, clearBanNotification, setServers } = useAuth();
  
  const [selectedServer, setSelectedServer] = useState<Server | null>(null);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);

  // NOUVEAUX ÉTATS POUR GÉRER L'ONGLET ET LE DM
  const [activeTab, setActiveTab] = useState<"servers" | "dms">("servers");
  const [activeDMUser, setActiveDMUser] = useState<{ id: string; name: string } | null>(null);

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

  // Action : Quand on clique sur un serveur
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

  // Action : Quand on clique sur un salon
  const handleChannelSelect = (channel: Channel | null) => { // <-- Ajout de | null
    setSelectedChannel(channel);
    setActiveDMUser(null);    // On quitte le mode DM
    
    // Si channel est null, on s'arrête là (ce qui arrive quand on change d'onglet)
    if (!channel) return;

    // Si on a un salon mais pas de serveur sélectionné (ou mauvais serveur), 
    // il faut s'assurer que selectedServer est bien défini pour la MembersBar
    if (channel.server_id && (!selectedServer || selectedServer.id !== channel.server_id)) {
      // Ici, on part du principe que ton objet Channel contient le server_id
      setSelectedServer({ id: channel.server_id } as Server);
    }
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
      <Userbar username={username} onStatusChange={setUserStatus} />

      {/* Notification ban/expulsion */}
      {banNotification && (
        <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/70">
          <div className="bg-[#181825] border border-red-600 rounded-2xl p-8 max-w-sm w-full mx-4 shadow-2xl text-center">
            <div className="text-4xl mb-4">
              {banNotification.message.startsWith("\u274c") ? "\ud83d\udd28" : banNotification.message.startsWith("\u23f3") ? "\u23f3" : "\ud83d\udeaa"}
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
      
      <ChatBar 
        onServerSelect={handleServerSelect} 
        onChannelSelect={handleChannelSelect} 
        activeTab={activeTab} 
        setActiveTab={setActiveTab} 
        onDMSelect={handleStartDM} 
      />
      
      {/* On n'affiche la barre des membres que sur l'onglet Serveurs */}
      {activeTab === "servers" && (
        <MembersBar 
          userStatus={userStatus} 
          selectedServer={selectedServer} 
          selectedChannel={selectedChannel} 
          onStartDM={handleStartDM} 
        />
      )}

      <Chat 
        selectedServer={selectedServer} 
        selectedChannel={selectedChannel} 
        activeDMUser={activeDMUser}
      />
      
      {children}
    </>
  );
}
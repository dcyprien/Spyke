"use client";

import Navbar from "../../components/navbar";
import Mainbar from "../../components/mainbar";
import Userbar from "../../components/userbar";
import ChatBar, { Server, Channel } from "../../components/chatbar";
import MembersBar from "../../components/membersbar";
import Chat from "../../components/chat";
import { useEffect, useState } from "react";
import { useAuth } from "../context";
import { useLang } from "../langContext";

type UserStatus = "online" | "offline" | "invisible";

export default function MainLayout({ children }: { children: React.ReactNode }) {
  const [username, setUsername] = useState<string>("");
  const [userStatus, setUserStatus] = useState<UserStatus>("online");
  const { banNotifications, dismissBanNotification, setServers } = useAuth();
  const { t } = useLang();
  
  const [selectedServer, setSelectedServer] = useState<Server | null>(null);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);

  // Si on est banni/expuls\u00e9 du serveur actuellement s\u00e9lectionn\u00e9, le d\u00e9s\u00e9lectionner
  useEffect(() => {
    const first = banNotifications[0];
    if (first && selectedServer && selectedServer.id === first.serverId) {
      setSelectedServer(null);
      setSelectedChannel(null);
    }
  }, [banNotifications]);

  useEffect(() => {
    const storedUsername = localStorage.getItem("username");
    if (storedUsername) setUsername(storedUsername);
  }, []);

  // Action : Quand on clique sur un serveur
  const handleServerSelect = (server: Server) => {
    setSelectedServer(server);
    setSelectedChannel(null); // On reset le salon pour forcer l'utilisateur à en choisir un
  };

  // Action : Quand on clique sur un salon
  const handleChannelSelect = (channel: Channel) => {
    setSelectedChannel(channel);
    // Si on a un salon mais pas de serveur sélectionné (ou mauvais serveur), 
    // il faut s'assurer que selectedServer est bien défini pour la MembersBar
    if (channel.server_id && (!selectedServer || selectedServer.id !== channel.server_id)) {
      // Ici, on part du principe que ton objet Channel contient le server_id
      setSelectedServer({ id: channel.server_id } as Server);
    }
  };

  return (
    <>
      <Navbar selectedServer={selectedServer} />
      <Mainbar />
      <Userbar username={username} onStatusChange={setUserStatus} />

      {/* Notification ban/expulsion - shows the first pending notification */}
      {banNotifications.length > 0 && (() => {
        const notif = banNotifications[0];
        return (
          <div className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/70">
            <div className="bg-[#181825] border border-red-600 rounded-2xl p-8 max-w-sm w-full mx-4 shadow-2xl text-center">
              <div className="text-4xl mb-4">
                {notif.message.startsWith("❌") ? "🔨" : notif.message.startsWith("⏳") ? "⏳" : "🚪"}
              </div>
              <p className="text-white font-semibold text-lg mb-2">{t.layout_ban_title}</p>
              <p className="text-gray-300 text-sm mb-6">{notif.message}</p>
              <button
                onClick={dismissBanNotification}
                className="px-6 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg font-bold transition"
              >
                {t.layout_ban_ok}
              </button>
            </div>
          </div>
        );
      })()}
      
      <ChatBar 
        onServerSelect={handleServerSelect} 
        onChannelSelect={handleChannelSelect} // Utilise la nouvelle fonction
        username={username} 
      />
      
      <MembersBar 
        userStatus={userStatus} 
        selectedServer={selectedServer} 
        selectedChannel={selectedChannel} // Prop cruciale
      />

      <Chat 
        selectedServer={selectedServer} 
        selectedChannel={selectedChannel} 
      />
      
      {children}
    </>
  );
}
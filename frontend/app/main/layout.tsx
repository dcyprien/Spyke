"use client";

import Navbar from "../../components/navbar";
import Mainbar from "../../components/mainbar";
import Userbar from "../../components/userbar";
import ChatBar, { Server, Channel } from "../../components/chatbar";
import MembersBar from "../../components/membersbar";
import Chat from "../../components/chat";
import { useEffect, useState } from "react";

type UserStatus = "online" | "away" | "offline" | "invisible";

export default function MainLayout({ children }: { children: React.ReactNode }) {
  const [username, setUsername] = useState<string>("");
  const [userStatus, setUserStatus] = useState<UserStatus>("online");
  
  const [selectedServer, setSelectedServer] = useState<Server | null>(null);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);

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
"use client";

import Image from "next/image";
import { useState, useRef, useEffect } from "react";

type UserStatus = "online" | "away" | "offline" | "invisible";

const statusConfig: Record<UserStatus, { label: string; color: string }> = {
  online: { label: "Online", color: "bg-green" },
  offline: { label: "Offline", color: "bg-red" },
  away: { label: "Away", color: "bg-yellow" },
  invisible: { label: "Invisible", color: "bg-grey-light" },
};

type Props = {
  username?: string;
  onStatusChange?: (status: UserStatus) => void;
};

export default function UserControlPanel({ username: initialUsername, onStatusChange }: Props) {
  const [status, setStatus] = useState<UserStatus>("online");
  const [statusOpen, setStatusOpen] = useState(false);

  const [avatar, setAvatar] = useState("/images/user.png");
  const [username, setUsername] = useState<string>(initialUsername || "");

  useEffect(() => {
    if (!username) {
      const storedUsername = localStorage.getItem("username");
      if (storedUsername) {
        setUsername(storedUsername);
      }
    }
    
    // Restaurer le statut depuis localStorage
    const storedStatus = localStorage.getItem("userStatus") as UserStatus | null;
    if (storedStatus) {
      setStatus(storedStatus);
    }
  }, [username]);

  return (
 <div className="
  fixed z-50 bottom-2 left-3
  bg-dark-navy
  border border-grey-light
  rounded-xl px-3 py-2 flex items-center space-x-2
  shadow-[0_4px_8px_rgba(0,0,0,0.4)]
">


      {/* Avatar + Status */}
      <div className="flex items-center space-x-2">

        {/* Avatar */}
        <div
          className="relative cursor-pointer"
        >
          <Image
            src={avatar}
            alt="User"
            width={48}
            height={48}
            className="rounded-full"
          />

        </div>

        {/* Bouton statut */}
        <div className="relative">
          <button
            onClick={() => setStatusOpen(!statusOpen)}
            className="
              w-10 h-10
              rounded-xl
              flex items-center justify-center
              transition
              hover:bg-blue-mid
              hover:shadow-md
            "
          >
            <span className={`w-3 h-3 rounded-full ${statusConfig[status].color}`} />
          </button>

          {/* Dropdown statut */}
          {statusOpen && (
            <div
            className="
                absolute
                bottom-full   /* place le menu au-dessus du bouton */
                right-15        /* aligne à droite du bouton */
                mb-2           /* petit écart entre bouton et menu */
                w-40
                bg-grey
                rounded-xl
                shadow-lg
                border border-white/20
                z-1
                "
            >
              <ul className="p-2 space-y-1">
                {(Object.keys(statusConfig) as UserStatus[]).map((key) => (
                  <li key={key}>
                    <button
                      onClick={() => {
                        setStatus(key);
                        localStorage.setItem("userStatus", key);
                        if (onStatusChange) onStatusChange(key);
                        setStatusOpen(false);
                      }}
                      className="
                        w-full flex items-center gap-2
                        px-3 py-2
                        rounded-lg
                        text-white text-sm
                        hover:bg-grey-light
                        transition
                      "
                    >
                      <span className={`w-3 h-3 rounded-full ${statusConfig[key].color}`} />
                      {statusConfig[key].label}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>

      </div>

      {/* Username */}
      <div className="flex flex-col ml-2">
         <span className="text-sm font-semibold text-[cream]">
            {username || "User"}
        </span>
    </div>
    </div>
  );
}

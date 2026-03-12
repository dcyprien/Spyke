import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./components/**/*.{js,ts,jsx,tsx,mdx}",
    "./app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "var(--background)",
        foreground: "var(--foreground)",
         "dark-navy": "#0A0F27",   // Couleur principale / background
        "dark-blue": "#001839",
        "navy-deep": "#001952",   // Couleur secondaire foncée
        "blue-mid": "#043775",    // Couleur intermédiaire
        "cyan": "#12769E",        // Couleur accent / call-to-action
        "cream": "#FCF6F3",       // Couleur claire / texte secondaire
        "blue-gray": "#8F91B4",  // Couleur neutre  / texte / background secondaire
        "grey": "#3D3D3D", //bouton 
        "grey-light": "#666666", // bouton
        "green": "#0BDA51", // actif status
        "red": "#9F1717", //  off status
        "yellow": "#E5E500", // away status
      },
    },
  },
  safelist: [
    "bg-green-500",
    "bg-gray-500",
    "bg-red-500",
    "bg-yellow-500",
    "bg-amber-500",
  ],
  plugins: [],
};
export default config;

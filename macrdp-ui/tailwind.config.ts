import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        macos: {
          bg: "#f5f5f7",
          card: "#ffffff",
          border: "#d2d2d7",
          text: "#1d1d1f",
          secondary: "#86868b",
          blue: "#007aff",
          green: "#34c759",
          red: "#ff3b30",
          orange: "#ff9500",
        },
      },
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Text",
          "SF Pro Display",
          "Helvetica Neue",
          "Arial",
          "sans-serif",
        ],
      },
    },
  },
  plugins: [],
} satisfies Config;

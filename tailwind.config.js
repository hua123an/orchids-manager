/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        background: '#f8f9fb', // Very light gray/blueish background
        surface: '#ffffff',    // Pure white cards
        primary: {
           DEFAULT: '#2563eb', // Royal Blue
           hover: '#1d4ed8',
        },
        text: {
           main: '#18181b',    // Almost black
           sub: '#71717a',     // Gray 500
        },
        border: '#e4e4e7',     // Zinc 200
        
        // Status colors
        success: '#10b981', 
        warning: '#f59e0b',
        error: '#ef4444',
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
      },
      boxShadow: {
         'card': '0 1px 2px 0 rgba(0, 0, 0, 0.05)',
         'card-hover': '0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -2px rgba(0, 0, 0, 0.05)',
         'glow': '0 0 15px rgba(37, 99, 235, 0.2)',
      }
    },
  },
  plugins: [],
}

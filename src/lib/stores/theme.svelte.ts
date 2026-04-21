const themes = {
  dark: {
    bg: "transparent", sidebarBg: "rgba(22, 27, 34, 0.75)", termBg: "rgba(13, 17, 23, 0.85)",
    border: "#30363d", textPrimary: "#e6edf3", textSecondary: "#8b949e",
    termTheme: {
      background: "#0d1117", foreground: "#e6edf3", cursor: "#58a6ff", cursorAccent: "#0d1117",
      selectionBackground: "rgba(88, 166, 255, 0.3)",
      black: "#484f58", red: "#ff7b72", green: "#3fb950", yellow: "#d29922",
      blue: "#58a6ff", magenta: "#bc8cff", cyan: "#39d353", white: "#b1bac4",
      brightBlack: "#6e7681", brightRed: "#ffa198", brightGreen: "#56d364",
      brightYellow: "#e3b341", brightBlue: "#79c0ff", brightMagenta: "#d2a8ff",
      brightCyan: "#56d364", brightWhite: "#f0f6fc",
    }
  },
  light: {
    bg: "transparent", sidebarBg: "rgba(246, 248, 250, 0.8)", termBg: "rgba(255, 255, 255, 0.9)",
    border: "#d0d7de", textPrimary: "#1f2328", textSecondary: "#656d76",
    termTheme: {
      background: "#ffffff", foreground: "#1f2328", cursor: "#0969da", cursorAccent: "#ffffff",
      selectionBackground: "rgba(9, 105, 218, 0.2)",
      black: "#24292f", red: "#cf222e", green: "#116329", yellow: "#4d2d00",
      blue: "#0969da", magenta: "#8250df", cyan: "#1b7c83", white: "#6e7781",
      brightBlack: "#57606a", brightRed: "#a40e26", brightGreen: "#1a7f37",
      brightYellow: "#633c01", brightBlue: "#218bff", brightMagenta: "#a475f9",
      brightCyan: "#3192aa", brightWhite: "#8c959f",
    }
  }
} as const;

export type ThemeName = keyof typeof themes;

class ThemeStore {
  currentTheme = $state<ThemeName>('dark');
  accentColor = $state('#58a6ff');

  constructor() {
    if (typeof localStorage !== 'undefined') {
      this.currentTheme = (localStorage.getItem('clauge-theme') as ThemeName) || 'dark';
      this.accentColor = localStorage.getItem('clauge-accent') || '#58a6ff';
    }
  }

  /** Returns the termTheme so callers can update xterm instances. */
  applyTheme(themeName: ThemeName): { termTheme: (typeof themes)[ThemeName]['termTheme']; cursor: string } {
    this.currentTheme = themeName;
    localStorage.setItem('clauge-theme', themeName);
    const t = themes[themeName];
    const root = document.documentElement;
    root.style.setProperty('--sidebar-bg', t.sidebarBg);
    root.style.setProperty('--term-bg', t.termBg);
    root.style.setProperty('--border', t.border);
    root.style.setProperty('--text-primary', t.textPrimary);
    root.style.setProperty('--text-secondary', t.textSecondary);
    root.style.setProperty('--accent', this.accentColor);
    root.style.setProperty('--modal-bg', themeName === 'light' ? 'rgba(255, 255, 255, 0.95)' : '#161b22');
    root.style.setProperty('--input-bg', themeName === 'light' ? '#f6f8fa' : '#0d1117');
    root.style.setProperty('--hover-bg', themeName === 'light' ? 'rgba(0,0,0,0.04)' : 'rgba(255,255,255,0.06)');
    root.style.setProperty('--btn-bg', themeName === 'light' ? '#f0f2f4' : '#21262d');
    return { termTheme: t.termTheme, cursor: this.accentColor };
  }

  /** Returns the combined termTheme so callers can update xterm instances. */
  applyAccent(color: string): { termTheme: (typeof themes)[ThemeName]['termTheme']; cursor: string } {
    this.accentColor = color;
    localStorage.setItem('clauge-accent', color);
    document.documentElement.style.setProperty('--accent', color);
    const termTheme = { ...themes[this.currentTheme].termTheme, cursor: color };
    return { termTheme, cursor: color };
  }

  /** Get the termTheme for the current theme + accent (for initialising new terminals). */
  getTermTheme() {
    return { ...themes[this.currentTheme].termTheme, cursor: this.accentColor };
  }
}

export const theme = new ThemeStore();

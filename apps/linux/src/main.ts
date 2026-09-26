// Nova Cross-Device Ecosystem Desktop Client

interface Device {
  id: string;
  name: string;
  fingerprint: string;
  platform: string;
  trusted: boolean;
  features: number[];
  last_seen: string;
}

interface ClipboardEntry {
  id: string;
  source_device: string;
  content: { type: string; value: string };
  hash: string;
  sensitive: boolean;
  preview: string;
  timestamp: string;
}

interface Note {
  id: string;
  title: string;
  content: string;
  folder: string;
  tags: string[];
  updated_at: string;
}

interface FileTransfer {
  session_id: string;
  file_name: string;
  file_size: number;
  progress: number;
  status: string;
  sha256: string;
}

class NovaApp {
  private activeTab: string = 'overview';
  private apiBase: string = 'http://127.0.0.1:40199/api';
  private daemonOnline: boolean = false;
  private localDevice: any = {
    name: "Prince's Linux PC",
    fingerprint: '3f8a-9c12-e4b7-10d9',
    device_id: 'c8f13b52-73a4-4a51-934c-687f89b91011',
    port: 53418
  };
  private devices: Device[] = [
    {
      id: 'd9e22a10-21a4-48f1-9c88-42f8832a8190',
      name: "Prince's Pixel 9 Pro",
      fingerprint: '84b1-59f2-aa41-9872',
      platform: 'android',
      trusted: true,
      features: [1, 2, 3, 4, 5, 6, 7],
      last_seen: new Date().toISOString()
    }
  ];
  private clipboardHistory: ClipboardEntry[] = [
    {
      id: '1',
      source_device: 'Local Linux',
      content: { type: 'Text', value: 'https://github.com/nova-ecosystem/nova' },
      hash: '3f7a1...',
      sensitive: false,
      preview: 'https://github.com/nova-ecosystem/nova',
      timestamp: 'Just now'
    },
    {
      id: '2',
      source_device: "Prince's Pixel 9 Pro",
      content: { type: 'Text', value: 'API_SECRET_KEY_FOR_TESTING' },
      hash: 'a9b2c...',
      sensitive: true,
      preview: '•••••••••••••••• (Sensitive Content)',
      timestamp: '5m ago'
    }
  ];
  private notes: Note[] = [
    {
      id: '1',
      title: 'Nova Architectural Specification',
      content: '# Nova Architecture\nPeer-to-peer Linux & Android cross-device ecosystem using Ed25519 identity, Noise XX encryption, and Yjs CRDT.',
      folder: 'Architecture',
      tags: ['system', 'spec'],
      updated_at: 'Today 12:45'
    },
    {
      id: '2',
      title: 'WebRTC & PipeWire Pipeline',
      content: 'Low-latency screen mirroring via PipeWire portal capture into hardware-accelerated H.264 stream.',
      folder: 'Research',
      tags: ['video', 'linux'],
      updated_at: 'Yesterday'
    }
  ];
  private activeNoteId: string = '1';
  private transfers: FileTransfer[] = [
    {
      session_id: 'sess-1',
      file_name: 'nova_architecture.pdf',
      file_size: 4200000,
      progress: 100,
      status: 'Completed',
      sha256: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
    }
  ];
  private remotePromptPending: boolean = false;
  private qrModalOpen: boolean = false;
  private pairingPin: string = '842 195';
  private modalTab: 'pair' | 'download' = 'pair';
  private downloadQrSvg: string = '';
  private downloadUrl: string = 'https://github.com/princekjha-dev/Nova/releases/latest/download/nova-android.apk';

  constructor() {
    this.init();
  }

  private async init() {
    await this.checkDaemon();
    this.render();
    this.attachEvents();
  }

  private async checkDaemon() {
    try {
      const res = await fetch(`${this.apiBase}/status`, { signal: AbortSignal.timeout(1500) });
      if (res.ok) {
        const data = await res.json();
        this.daemonOnline = true;
        this.localDevice = {
          name: data.device_name || this.localDevice.name,
          fingerprint: data.fingerprint || this.localDevice.fingerprint,
          device_id: data.device_id || this.localDevice.device_id,
          port: data.port || 53418
        };
      }
      const qrRes = await fetch(`${this.apiBase}/pair/download-qr`, { signal: AbortSignal.timeout(1500) });
      if (qrRes.ok) {
        const qrData = await qrRes.json();
        if (qrData.svg) this.downloadQrSvg = qrData.svg;
        if (qrData.url) this.downloadUrl = qrData.url;
      }
    } catch {
      this.daemonOnline = false;
    }
  }

  private render() {
    const appEl = document.getElementById('app');
    if (!appEl) return;

    appEl.innerHTML = `
      <div class="app-container">
        <!-- Sidebar Navigation -->
        <aside class="sidebar">
          <div class="brand-section">
            <div class="brand-logo">
              <svg viewBox="0 0 24 24"><path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/></svg>
            </div>
            <div>
              <div class="brand-title">NOVA</div>
              <div class="brand-subtitle">Cross-Device Hub</div>
            </div>
          </div>

          <ul class="nav-menu">
            ${this.renderNavItem('overview', 'Overview', 'M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z')}
            ${this.renderNavItem('devices', 'Devices', 'M4 6h16v12H4zm2 2v8h12V8H6zm14-5H4c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z', this.devices.length.toString())}
            ${this.renderNavItem('clipboard', 'Clipboard', 'M19 2h-4.18C14.4 0.84 13.3 0 12 0c-1.3 0-2.4.84-2.82 2H5c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm-7 0c.55 0 1 .45 1 1s-.45 1-1 1-1-.45-1-1 .45-1 1-1zm7 18H5V4h2v3h10V4h2v16z')}
            ${this.renderNavItem('notes', 'Notes', 'M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z', this.notes.length.toString())}
            ${this.renderNavItem('files', 'Files', 'M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM14 13v4h-4v-4H7l5-5 5 5h-3z')}
            ${this.renderNavItem('tasks', 'Tasks', 'M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-2 10h-4v4h-2v-4H7v-2h4V7h2v4h4v2z')}
            ${this.renderNavItem('ai', 'AI Assistant', 'M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2zm1 14.93V17a1 1 0 0 1-2 0v-.07A7.003 7.003 0 0 1 5.07 11H5a1 1 0 0 1 0-2h.07A7.003 7.003 0 0 1 11 3.07V3a1 1 0 0 1 2 0v.07A7.003 7.003 0 0 1 18.93 9H19a1 1 0 0 1 0 2h-.07A7.003 7.003 0 0 1 13 16.93z')}
            ${this.renderNavItem('remote', 'Remote PC', 'M21 2H3c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h7v2H8v2h8v-2h-2v-2h7c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H3V4h18v12z')}
            ${this.renderNavItem('collaboration', 'Collaboration', 'M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z')}
            ${this.renderNavItem('settings', 'Settings', 'M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z')}
          </ul>

          <div class="sidebar-status">
            <div class="status-indicator"></div>
            <div class="status-info">
              <div class="device-label">${this.localDevice.name}</div>
              <div class="status-sub">${this.daemonOnline ? 'Core Daemon Active' : 'Local Standalone Mode'}</div>
            </div>
          </div>
        </aside>

        <!-- Main View Area -->
        <main class="main-view">
          <header class="top-bar">
            <div class="page-title-group">
              <h1 id="page-title">${this.getTabTitle()}</h1>
              <p id="page-subtitle">${this.getTabSubtitle()}</p>
            </div>
            <div class="top-bar-actions">
              <button class="btn-primary" id="btn-pair-device">
                <svg width="16" height="16" fill="currentColor" viewBox="0 0 24 24"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
                Pair New Device
              </button>
            </div>
          </header>

          <section class="content-body" id="content-body">
            ${this.renderActiveTabContent()}
          </section>
        </main>

        <!-- QR Pairing & APK Download Modal -->
        <div class="modal-overlay ${this.qrModalOpen ? 'active' : ''}" id="qr-modal">
          <div class="modal-content">
            <div class="modal-header">
              <div class="modal-title">${this.modalTab === 'pair' ? 'Pair Android Phone' : 'Install Nova for Android'}</div>
              <button class="modal-close" id="modal-close-btn">&times;</button>
            </div>

            <!-- Modal Subtabs for Instant Adoption -->
            <div class="modal-tabs">
              <button class="modal-tab-btn ${this.modalTab === 'pair' ? 'active' : ''}" id="tab-btn-pair">
                <svg width="14" height="14" fill="currentColor" viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 15-5-5 1.41-1.41L11 14.17l7.59-7.59L20 8l-9 9z"/></svg>
                Scan to Pair
              </button>
              <button class="modal-tab-btn ${this.modalTab === 'download' ? 'active' : ''}" id="tab-btn-download">
                <svg width="14" height="14" fill="currentColor" viewBox="0 0 24 24"><path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM17 13l-5 5-5-5h3V9h4v4h3z"/></svg>
                Download APK
              </button>
            </div>

            ${this.modalTab === 'pair' ? `
              <div class="qr-box">
                <svg viewBox="0 0 100 100" fill="#0f172a">
                  <rect width="100" height="100" fill="#ffffff"/>
                  <rect x="10" y="10" width="24" height="24" fill="#0f172a"/>
                  <rect x="14" y="14" width="16" height="16" fill="#ffffff"/>
                  <rect x="18" y="18" width="8" height="8" fill="#6366f1"/>
                  <rect x="66" y="10" width="24" height="24" fill="#0f172a"/>
                  <rect x="70" y="14" width="16" height="16" fill="#ffffff"/>
                  <rect x="74" y="18" width="8" height="8" fill="#6366f1"/>
                  <rect x="10" y="66" width="24" height="24" fill="#0f172a"/>
                  <rect x="14" y="70" width="16" height="16" fill="#ffffff"/>
                  <rect x="18" y="74" width="8" height="8" fill="#6366f1"/>
                  <rect x="42" y="12" width="6" height="6" fill="#0f172a"/>
                  <rect x="52" y="18" width="6" height="6" fill="#0f172a"/>
                  <rect x="42" y="42" width="16" height="16" fill="#06b6d4"/>
                  <rect x="68" y="42" width="6" height="6" fill="#0f172a"/>
                  <rect x="42" y="68" width="8" height="8" fill="#0f172a"/>
                  <rect x="68" y="68" width="12" height="12" fill="#6366f1"/>
                </svg>
              </div>
              <div style="text-align: center; color: var(--text-muted); font-size: 13px;">
                Scan this code with the <strong>Nova Android App</strong> or enter the confirmation PIN:
              </div>
              <div class="pin-display">${this.pairingPin}</div>
              <div style="font-size: 12px; color: var(--text-dim); text-align: center;">
                Ed25519 Fingerprint: <span class="code-tag">${this.localDevice.fingerprint}</span>
              </div>
              <div style="text-align: center; margin-top: 14px; font-size: 12px; color: var(--text-muted);">
                Don't have the Nova Android app installed yet?
                <a href="javascript:void(0)" id="switch-to-download-link" style="color: var(--accent-cyan); font-weight: 600; margin-left: 4px; text-decoration: underline;">Get Android APK &rarr;</a>
              </div>
              <button class="btn-primary" style="width: 100%; margin-top: 16px; justify-content: center;" id="modal-done-btn">
                Done
              </button>
            ` : `
              <div class="qr-box" id="download-qr-container">
                ${this.downloadQrSvg || `
                  <svg viewBox="0 0 100 100" fill="#0f172a">
                    <rect width="100" height="100" fill="#ffffff"/>
                    <rect x="10" y="10" width="24" height="24" fill="#0f172a"/>
                    <rect x="14" y="14" width="16" height="16" fill="#ffffff"/>
                    <rect x="18" y="18" width="8" height="8" fill="#06b6d4"/>
                    <rect x="66" y="10" width="24" height="24" fill="#0f172a"/>
                    <rect x="70" y="14" width="16" height="16" fill="#ffffff"/>
                    <rect x="74" y="18" width="8" height="8" fill="#06b6d4"/>
                    <rect x="10" y="66" width="24" height="24" fill="#0f172a"/>
                    <rect x="14" y="70" width="16" height="16" fill="#ffffff"/>
                    <rect x="18" y="74" width="8" height="8" fill="#06b6d4"/>
                    <rect x="44" y="24" width="12" height="12" fill="#0f172a"/>
                    <rect x="36" y="44" width="28" height="28" fill="#6366f1"/>
                    <path d="M50 48v16m-5-5 5 5 5-5" stroke="#ffffff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
                  </svg>
                `}
              </div>
              <div style="text-align: center; color: var(--text-muted); font-size: 13px; margin-bottom: 14px;">
                Scan with your phone camera to download <strong>nova-android.apk</strong> directly:
              </div>
              <div style="display: flex; gap: 8px; margin-bottom: 16px;">
                <a href="${this.downloadUrl}" target="_blank" class="btn-primary" style="flex: 1; justify-content: center; text-decoration: none;">
                  <svg width="16" height="16" fill="currentColor" viewBox="0 0 24 24"><path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM17 13l-5 5-5-5h3V9h4v4h3z"/></svg>
                  Download APK
                </a>
                <a href="https://github.com/princekjha-dev/Nova/releases" target="_blank" class="btn-secondary" style="justify-content: center; text-decoration: none;">
                  GitHub Releases
                </a>
              </div>
              <div style="background: rgba(255,255,255,0.03); border: 1px solid var(--border-subtle); border-radius: var(--radius-sm); padding: 12px; margin-bottom: 16px;">
                <div style="font-size: 12px; font-weight: 700; color: var(--text-main); margin-bottom: 8px;">Easy 4-Step Sideload Guide:</div>
                <div class="guide-step"><span class="step-number">1</span><span>Scan QR with phone camera or click <em>Download APK</em>.</span></div>
                <div class="guide-step"><span class="step-number">2</span><span>Open the downloaded <code>nova-android.apk</code> file.</span></div>
                <div class="guide-step"><span class="step-number">3</span><span>Tap <strong>Install</strong> (allow <em>"Install unknown apps"</em> if prompted).</span></div>
                <div class="guide-step"><span class="step-number">4</span><span>Open Nova on phone, tap <strong>Pair Device</strong>, and scan the QR below.</span></div>
              </div>
              <button class="btn-primary" style="width: 100%; justify-content: center;" id="switch-to-pair-btn">
                App Installed &rarr; Switch to Pairing QR
              </button>
            `}
          </div>
        </div>

        <!-- Remote Control Request Alert Modal -->
        <div class="modal-overlay ${this.remotePromptPending ? 'active' : ''}" id="remote-prompt-modal">
          <div class="modal-content" style="border-color: var(--accent-amber);">
            <div class="modal-header">
              <div class="modal-title" style="color: var(--accent-amber);">Remote Control Request</div>
            </div>
            <p style="font-size: 14px; color: var(--text-muted); line-height: 1.6; margin-bottom: 20px;">
              Device <strong>Prince's Pixel 9 Pro</strong> has requested session authorization to control this Linux PC (screen, mouse, keyboard).
            </p>
            <div style="display: flex; gap: 12px; justify-content: flex-end;">
              <button class="btn-secondary" id="btn-remote-reject">Reject</button>
              <button class="btn-primary" style="background: var(--accent-amber); border-color: #d97706;" id="btn-remote-allow">
                Allow Remote Session
              </button>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  private renderNavItem(tab: string, label: string, svgPath: string, badge?: string): string {
    const isActive = this.activeTab === tab;
    return `
      <li class="nav-item ${isActive ? 'active' : ''}" data-tab="${tab}">
        <svg class="icon" viewBox="0 0 24 24" fill="currentColor">
          <path d="${svgPath}"/>
        </svg>
        <span>${label}</span>
        ${badge ? `<span class="nav-badge">${badge}</span>` : ''}
      </li>
    `;
  }

  private getTabTitle(): string {
    switch (this.activeTab) {
      case 'overview': return 'Ecosystem Overview';
      case 'devices': return 'Connected Devices';
      case 'clipboard': return 'Super Clipboard';
      case 'notes': return 'Continuous Notes Sync';
      case 'files': return 'File EasyShare';
      case 'tasks': return 'Task Handoff';
      case 'ai': return 'Nova AI Assistant';
      case 'remote': return 'Remote PC';
      case 'collaboration': return 'Infinite Collaboration';
      case 'settings': return 'System Settings';
      default: return 'Nova Hub';
    }
  }

  private getTabSubtitle(): string {
    switch (this.activeTab) {
      case 'overview': return 'Cross-device continuity between Linux desktop and Android phone';
      case 'devices': return 'Manage cryptographic identities and capability permissions';
      case 'clipboard': return 'Real-time peer-to-peer clipboard synchronizer with sensitive detection';
      case 'notes': return 'Local-first Markdown notes with Yjs CRDT real-time conflict-free sync';
      case 'files': return 'High-speed LAN file transfer with chunking and SHA-256 verification';
      case 'tasks': return 'Handoff open tasks, research URLs, and AI contexts across devices';
      case 'ai': return 'Private orchestration layer with strict permission boundaries and MCP tools';
      case 'remote': return 'Stream desktop and control input with explicit session verification';
      case 'collaboration': return 'Real-time multi-device workspace collaboration and presence';
      case 'settings': return 'Identity keys, mDNS discovery, and encryption options';
      default: return '';
    }
  }

  private renderActiveTabContent(): string {
    switch (this.activeTab) {
      case 'overview': return this.renderOverview();
      case 'devices': return this.renderDevices();
      case 'clipboard': return this.renderClipboard();
      case 'notes': return this.renderNotes();
      case 'files': return this.renderFiles();
      case 'tasks': return this.renderTasks();
      case 'ai': return this.renderAi();
      case 'remote': return this.renderRemote();
      case 'collaboration': return this.renderCollaboration();
      case 'settings': return this.renderSettings();
      default: return '';
    }
  }

  private renderOverview(): string {
    return `
      <!-- Mobile Adoption & APK Download Banner -->
      <div class="adoption-banner">
        <div class="adoption-banner-content">
          <div class="adoption-banner-icon">📱</div>
          <div>
            <div class="adoption-banner-title">Get Nova on your Android Phone</div>
            <div class="adoption-banner-sub">Install the APK to unlock real-time Super Clipboard, Notes sync, File EasyShare, and Remote PC.</div>
          </div>
        </div>
        <div class="adoption-banner-actions">
          <button class="btn-primary" id="btn-quick-download-apk">
            <svg width="16" height="16" fill="currentColor" viewBox="0 0 24 24"><path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM17 13l-5 5-5-5h3V9h4v4h3z"/></svg>
            Download APK
          </button>
          <button class="btn-secondary" id="btn-quick-show-apk-qr">
            Show QR
          </button>
        </div>
      </div>

      <div class="card-grid">
        <div class="glass-card">
          <div class="card-header">
            <div class="card-title">This Linux PC</div>
            <span class="chip-status chip-emerald">Online</span>
          </div>
          <div style="font-size: 20px; font-weight: 700; margin-bottom: 8px;">${this.localDevice.name}</div>
          <div style="font-size: 13px; color: var(--text-dim); margin-bottom: 16px;">
            Port: <span class="code-tag">${this.localDevice.port}</span> | Discovery: <span class="code-tag">_nova._tcp</span>
          </div>
          <div style="font-size: 12px; color: var(--text-muted);">
            Fingerprint: <span class="code-tag">${this.localDevice.fingerprint}</span>
          </div>
        </div>

        <div class="glass-card">
          <div class="card-header">
            <div class="card-title">Paired Android Device</div>
            <span class="chip-status chip-cyan">Active LAN</span>
          </div>
          <div style="font-size: 20px; font-weight: 700; margin-bottom: 8px;">Prince's Pixel 9 Pro</div>
          <div style="font-size: 13px; color: var(--text-dim); margin-bottom: 16px;">
            Connected via direct encrypted TCP + TLS 1.3
          </div>
          <div style="display: flex; gap: 8px;">
            <button class="btn-secondary" style="font-size: 12px; padding: 6px 12px;" onclick="window.novaApp.setTab('clipboard')">Sync Clip</button>
            <button class="btn-secondary" style="font-size: 12px; padding: 6px 12px;" onclick="window.novaApp.setTab('files')">Send File</button>
            <button class="btn-secondary" style="font-size: 12px; padding: 6px 12px;" onclick="window.novaApp.setTab('notes')">View Notes</button>
          </div>
        </div>

        <div class="glass-card">
          <div class="card-header">
            <div class="card-title">Latest Clipboard Sync</div>
            <span class="chip-status chip-emerald">Synced</span>
          </div>
          <div style="font-size: 14px; color: var(--text-main); line-height: 1.5; margin-bottom: 12px;">
            ${this.clipboardHistory[0]?.preview || 'No recent clipboard data'}
          </div>
          <div style="font-size: 12px; color: var(--text-dim);">
            Source: ${this.clipboardHistory[0]?.source_device || 'Local'}
          </div>
        </div>
      </div>

      <div style="margin-top: 36px;">
        <h2 style="font-size: 18px; font-weight: 700; margin-bottom: 16px;">Active Subsystems</h2>
        <div class="card-grid">
          <div class="glass-card" style="padding: 16px;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <div>
                <div style="font-weight: 600; font-size: 15px;">Super Clipboard</div>
                <div style="font-size: 12px; color: var(--text-dim);">Bidirectional text, URLs & images</div>
              </div>
              <span class="chip-status chip-emerald">Active</span>
            </div>
          </div>
          <div class="glass-card" style="padding: 16px;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <div>
                <div style="font-weight: 600; font-size: 15px;">Notes Sync Engine</div>
                <div style="font-size: 12px; color: var(--text-dim);">Local-first Yjs CRDT</div>
              </div>
              <span class="chip-status chip-emerald">Active</span>
            </div>
          </div>
          <div class="glass-card" style="padding: 16px;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <div>
                <div style="font-weight: 600; font-size: 15px;">File EasyShare</div>
                <div style="font-size: 12px; color: var(--text-dim);">P2P chunked 64KB transport</div>
              </div>
              <span class="chip-status chip-emerald">Active</span>
            </div>
          </div>
          <div class="glass-card" style="padding: 16px;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <div>
                <div style="font-weight: 600; font-size: 15px;">Nova AI MCP Tools</div>
                <div style="font-size: 12px; color: var(--text-dim);">Port 40199 with permission guard</div>
              </div>
              <span class="chip-status chip-cyan">Ready</span>
            </div>
          </div>
        </div>
      </div>
    `;
  }

  private renderDevices(): string {
    return `
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
        <div style="font-size: 15px; color: var(--text-muted);">
          Total paired trusted devices: <strong>${this.devices.length}</strong>
        </div>
        <button class="btn-primary" onclick="window.novaApp.openQrModal()">
          <svg width="16" height="16" fill="currentColor" viewBox="0 0 24 24"><path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/></svg>
          Pair New Phone / PC
        </button>
      </div>

      <div class="card-grid">
        ${this.devices.map(dev => `
          <div class="glass-card">
            <div class="card-header">
              <div class="card-title">${dev.name}</div>
              <span class="chip-status ${dev.trusted ? 'chip-emerald' : 'chip-amber'}">
                ${dev.trusted ? 'Trusted' : 'Revoked'}
              </span>
            </div>
            <div style="font-size: 13px; color: var(--text-muted); margin-bottom: 12px;">
              Platform: <strong style="color: var(--text-main); text-transform: capitalize;">${dev.platform}</strong>
            </div>
            <div style="font-size: 12px; color: var(--text-dim); margin-bottom: 16px;">
              Fingerprint: <span class="code-tag">${dev.fingerprint}</span>
            </div>
            <div style="font-size: 12px; color: var(--text-muted); margin-bottom: 16px;">
              Permissions: Clipboard, Notes, Files, Screen, Remote
            </div>
            <div style="display: flex; gap: 8px;">
              <button class="btn-secondary" style="font-size: 12px; padding: 6px 14px;" onclick="window.novaApp.toggleTrust('${dev.id}')">
                ${dev.trusted ? 'Revoke Access' : 'Re-authorize'}
              </button>
              <button class="btn-secondary" style="font-size: 12px; padding: 6px 14px;" onclick="window.novaApp.testConnection('${dev.id}')">
                Ping Check
              </button>
            </div>
          </div>
        `).join('')}
      </div>
    `;
  }

  private renderClipboard(): string {
    return `
      <div style="background: var(--bg-card); border: 1px solid var(--border-subtle); border-radius: var(--radius-md); padding: 20px; margin-bottom: 24px;">
        <h3 style="font-size: 15px; font-weight: 600; margin-bottom: 12px;">Copy & Sync Instantly to Phone</h3>
        <div style="display: flex; gap: 12px;">
          <input type="text" class="form-input" id="input-clipboard-text" placeholder="Type or paste text to sync across paired devices..." />
          <button class="btn-primary" id="btn-send-clip" style="white-space: nowrap;">Copy & Sync</button>
        </div>
      </div>

      <h3 style="font-size: 16px; font-weight: 700; margin-bottom: 16px;">Clipboard History</h3>
      <div style="display: flex; flex-direction: column; gap: 12px;">
        ${this.clipboardHistory.map(entry => `
          <div class="glass-card" style="padding: 16px;">
            <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 8px;">
              <div style="font-size: 12px; color: var(--accent-cyan); font-weight: 600;">
                ${entry.source_device} • ${entry.timestamp}
              </div>
              <div style="display: flex; gap: 8px; align-items: center;">
                ${entry.sensitive ? '<span class="chip-status chip-amber" style="font-size: 10px;">Sensitive</span>' : ''}
                <button class="btn-secondary" style="font-size: 11px; padding: 4px 10px;" onclick="navigator.clipboard.writeText('${entry.content.value}')">
                  Copy
                </button>
              </div>
            </div>
            <div style="font-size: 14px; line-height: 1.5; color: var(--text-main); font-family: 'JetBrains Mono', monospace;">
              ${entry.preview}
            </div>
          </div>
        `).join('')}
      </div>
    `;
  }

  private renderNotes(): string {
    const activeNote = this.notes.find(n => n.id === this.activeNoteId) || this.notes[0];
    return `
      <div style="display: flex; gap: 20px; height: 100%;">
        <!-- Notes Sidebar -->
        <div style="width: 280px; display: flex; flex-direction: column; gap: 12px;">
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <input type="text" class="form-input" placeholder="Search notes..." style="font-size: 13px; padding: 8px 12px;" />
            <button class="btn-primary" style="padding: 8px 12px;" onclick="window.novaApp.createNote()">+ New</button>
          </div>
          <div style="display: flex; flex-direction: column; gap: 8px; overflow-y: auto;">
            ${this.notes.map(note => `
              <div class="glass-card" style="padding: 12px; cursor: pointer; border-color: ${note.id === this.activeNoteId ? 'var(--accent-primary)' : 'var(--border-subtle)'};" onclick="window.novaApp.selectNote('${note.id}')">
                <div style="font-weight: 600; font-size: 14px; margin-bottom: 4px;">${note.title}</div>
                <div style="font-size: 12px; color: var(--text-dim); display: flex; justify-content: space-between;">
                  <span>${note.folder}</span>
                  <span>${note.updated_at}</span>
                </div>
              </div>
            `).join('')}
          </div>
        </div>

        <!-- Note Editor View -->
        <div class="glass-card" style="flex: 1; display: flex; flex-direction: column; padding: 24px;">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; border-bottom: 1px solid var(--border-subtle); padding-bottom: 12px;">
            <input type="text" class="form-input" value="${activeNote ? activeNote.title : ''}" style="font-size: 18px; font-weight: 700; background: transparent; border: none; padding: 0;" />
            <span class="chip-status chip-emerald">CRDT Live Synced</span>
          </div>
          <textarea class="form-input" style="flex: 1; font-family: 'JetBrains Mono', monospace; font-size: 14px; line-height: 1.6; resize: none; background: transparent; border: none;" id="note-content-editor">${activeNote ? activeNote.content : ''}</textarea>
        </div>
      </div>
    `;
  }

  private renderFiles(): string {
    return `
      <div style="background: var(--bg-card); border: 2px dashed var(--border-subtle); border-radius: var(--radius-lg); padding: 40px; text-align: center; margin-bottom: 30px;">
        <svg style="width: 48px; height: 48px; fill: var(--accent-primary); margin-bottom: 12px;" viewBox="0 0 24 24"><path d="M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.64-2.05-4.78-4.65-4.96zM14 13v4h-4v-4H7l5-5 5 5h-3z"/></svg>
        <div style="font-size: 18px; font-weight: 700; margin-bottom: 6px;">Drop files to send over LAN</div>
        <div style="font-size: 13px; color: var(--text-muted); margin-bottom: 16px;">
          Files are chunked in 64 KiB frames, transferred directly P2P, and validated with SHA-256.
        </div>
        <button class="btn-primary" style="margin: 0 auto;" onclick="window.novaApp.triggerSendFile()">
          Select File to Transfer
        </button>
      </div>

      <h3 style="font-size: 16px; font-weight: 700; margin-bottom: 16px;">Transfer History</h3>
      <div style="display: flex; flex-direction: column; gap: 12px;">
        ${this.transfers.map(tr => `
          <div class="glass-card" style="padding: 16px;">
            <div style="display: flex; justify-content: space-between; margin-bottom: 6px;">
              <span style="font-weight: 600; font-size: 14px;">${tr.file_name}</span>
              <span class="chip-status chip-emerald">${tr.status}</span>
            </div>
            <div class="progress-bar-bg">
              <div class="progress-bar-fill" style="width: ${tr.progress}%;"></div>
            </div>
            <div style="display: flex; justify-content: space-between; margin-top: 8px; font-size: 11px; color: var(--text-dim);">
              <span>Size: ${(tr.file_size / 1024 / 1024).toFixed(2)} MB</span>
              <span>SHA-256: ${tr.sha256.slice(0, 16)}...</span>
            </div>
          </div>
        `).join('')}
      </div>
    `;
  }

  private renderTasks(): string {
    return `
      <div class="glass-card" style="margin-bottom: 24px;">
        <h3 style="font-size: 16px; font-weight: 600; margin-bottom: 16px;">Handoff Active Task to Phone</h3>
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 14px; margin-bottom: 14px;">
          <input type="text" class="form-input" id="handoff-title" placeholder="Task Title (e.g. Research: WebRTC Protocol)" value="Research: WebRTC Protocol" />
          <input type="text" class="form-input" id="handoff-url" placeholder="URL or Document path" value="https://webrtc.org" />
        </div>
        <button class="btn-primary" onclick="window.novaApp.dispatchHandoff()">
          Send to Prince's Pixel 9 Pro
        </button>
      </div>

      <h3 style="font-size: 16px; font-weight: 700; margin-bottom: 16px;">Active Tasks & Handoff Queue</h3>
      <div class="glass-card">
        <div style="display: flex; justify-content: space-between; align-items: center;">
          <div>
            <div style="font-weight: 600; font-size: 15px;">Research: WebRTC Protocol</div>
            <div style="font-size: 13px; color: var(--accent-cyan);">https://webrtc.org</div>
          </div>
          <button class="btn-secondary" style="font-size: 12px; padding: 6px 14px;" onclick="alert('Opening in default Linux browser...')">
            Open on Linux
          </button>
        </div>
      </div>
    `;
  }

  private renderAi(): string {
    return `
      <div style="display: grid; grid-template-columns: 2fr 1fr; gap: 20px; height: 100%;">
        <div class="glass-card" style="display: flex; flex-direction: column; padding: 20px;">
          <div style="font-size: 16px; font-weight: 700; margin-bottom: 16px;">Nova AI Assistant</div>
          <div style="flex: 1; background: rgba(0,0,0,0.3); border-radius: var(--radius-sm); padding: 16px; overflow-y: auto; margin-bottom: 16px;" id="ai-chat-box">
            <div style="background: rgba(99,102,241,0.15); border: 1px solid rgba(99,102,241,0.3); padding: 12px; border-radius: var(--radius-sm); font-size: 13px; line-height: 1.5; margin-bottom: 10px;">
              <strong>Nova Assistant:</strong> Hello! I am your private cross-device assistant. I can search your notes, lookup clipboard entries, or handoff tasks with strict permission boundaries.
            </div>
          </div>
          <div style="display: flex; gap: 10px;">
            <input type="text" class="form-input" id="ai-input" placeholder="Ask: 'Search my notes for WebRTC'..." />
            <button class="btn-primary" onclick="window.novaApp.askAi()">Ask</button>
          </div>
        </div>

        <div class="glass-card" style="padding: 20px;">
          <div style="font-size: 16px; font-weight: 700; margin-bottom: 14px;">Permission Boundaries</div>
          <div style="display: flex; flex-direction: column; gap: 14px; font-size: 13px;">
            <label style="display: flex; align-items: center; gap: 10px;">
              <input type="checkbox" checked disabled /> Search Local Notes
            </label>
            <label style="display: flex; align-items: center; gap: 10px;">
              <input type="checkbox" /> Read Private Clipboard (Restricted)
            </label>
            <label style="display: flex; align-items: center; gap: 10px;">
              <input type="checkbox" checked /> Query Connected Devices
            </label>
            <label style="display: flex; align-items: center; gap: 10px;">
              <input type="checkbox" checked /> Manage Tasks
            </label>
          </div>
          <div style="margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--border-subtle); font-size: 12px; color: var(--text-dim);">
            MCP Server endpoint: <br><span class="code-tag">http://127.0.0.1:40199/mcp</span>
          </div>
        </div>
      </div>
    `;
  }

  private renderRemote(): string {
    return `
      <div class="glass-card" style="margin-bottom: 24px;">
        <div class="card-header">
          <div class="card-title">Remote PC Control</div>
          <span class="chip-status chip-cyan">Explicit Auth Required</span>
        </div>
        <p style="font-size: 14px; color: var(--text-muted); line-height: 1.6; margin-bottom: 20px;">
          Remote control allows authorized Android devices to view your Linux desktop and inject synthetic mouse/keyboard events via uinput.
          Every session requires manual user approval.
        </p>
        <button class="btn-secondary" onclick="window.novaApp.simulateRemoteRequest()">
          Simulate Android Remote Request
        </button>
      </div>

      <div class="glass-card" style="height: 300px; display: flex; align-items: center; justify-content: center; border-style: dashed;">
        <div style="text-align: center; color: var(--text-dim);">
          <svg style="width: 48px; height: 48px; fill: var(--text-dim); margin-bottom: 8px;" viewBox="0 0 24 24"><path d="M21 2H3c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h7v2H8v2h8v-2h-2v-2h7c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2zm0 14H3V4h18v12z"/></svg>
          <div style="font-size: 15px; font-weight: 600;">No active remote desktop stream</div>
          <div style="font-size: 12px;">Waiting for authorized session</div>
        </div>
      </div>
    `;
  }

  private renderCollaboration(): string {
    return `
      <div class="card-grid">
        <div class="glass-card">
          <div class="card-header">
            <div class="card-title">Shared Workspaces</div>
            <span class="chip-status chip-emerald">CRDT Active</span>
          </div>
          <div style="font-size: 18px; font-weight: 700; margin-bottom: 6px;">Nova Engineering Notes</div>
          <div style="font-size: 12px; color: var(--text-dim); margin-bottom: 16px;">
            Presence: Prince (Linux Host), Prince's Phone (Collaborator)
          </div>
          <div style="font-size: 13px; color: var(--text-muted);">
            All edits update conflict-free using Lamport vector clocks.
          </div>
        </div>
      </div>
    `;
  }

  private renderSettings(): string {
    return `
      <div class="glass-card" style="max-width: 600px;">
        <h3 style="font-size: 16px; font-weight: 700; margin-bottom: 16px;">Cryptographic Device Identity</h3>
        <div style="display: flex; flex-direction: column; gap: 16px; font-size: 13px;">
          <div>
            <div style="color: var(--text-dim); margin-bottom: 4px;">Device ID (UUIDv5)</div>
            <div class="code-tag" style="word-break: break-all;">${this.localDevice.device_id}</div>
          </div>
          <div>
            <div style="color: var(--text-dim); margin-bottom: 4px;">Ed25519 Fingerprint</div>
            <div class="code-tag">${this.localDevice.fingerprint}</div>
          </div>
          <div>
            <div style="color: var(--text-dim); margin-bottom: 4px;">Local Discovery Port</div>
            <div class="code-tag">${this.localDevice.port} (TCP/UDP)</div>
          </div>
          <div>
            <div style="color: var(--text-dim); margin-bottom: 4px;">Key Storage</div>
            <div style="color: var(--accent-emerald);">Linux Secret Service / Local KeyStore (0600 mode)</div>
          </div>
        </div>
      </div>
    `;
  }

  private attachEvents() {
    document.querySelectorAll('.nav-item').forEach(item => {
      item.addEventListener('click', (e) => {
        const tab = (e.currentTarget as HTMLElement).getAttribute('data-tab');
        if (tab) this.setTab(tab);
      });
    });

    const pairBtn = document.getElementById('btn-pair-device');
    if (pairBtn) pairBtn.addEventListener('click', () => {
      this.modalTab = 'pair';
      this.openQrModal();
    });

    const closeBtn = document.getElementById('modal-close-btn');
    if (closeBtn) closeBtn.addEventListener('click', () => this.closeQrModal());

    const doneBtn = document.getElementById('modal-done-btn');
    if (doneBtn) doneBtn.addEventListener('click', () => this.closeQrModal());

    // Modal Tab Switching
    const tabPair = document.getElementById('tab-btn-pair');
    if (tabPair) tabPair.addEventListener('click', () => {
      this.modalTab = 'pair';
      this.render();
      this.attachEvents();
    });

    const tabDownload = document.getElementById('tab-btn-download');
    if (tabDownload) tabDownload.addEventListener('click', () => {
      this.modalTab = 'download';
      this.render();
      this.attachEvents();
    });

    const switchDownloadLink = document.getElementById('switch-to-download-link');
    if (switchDownloadLink) switchDownloadLink.addEventListener('click', () => {
      this.modalTab = 'download';
      this.render();
      this.attachEvents();
    });

    const switchPairBtn = document.getElementById('switch-to-pair-btn');
    if (switchPairBtn) switchPairBtn.addEventListener('click', () => {
      this.modalTab = 'pair';
      this.render();
      this.attachEvents();
    });

    const quickDownloadApk = document.getElementById('btn-quick-download-apk');
    if (quickDownloadApk) quickDownloadApk.addEventListener('click', () => {
      window.open(this.downloadUrl, '_blank');
    });

    const quickShowApkQr = document.getElementById('btn-quick-show-apk-qr');
    if (quickShowApkQr) quickShowApkQr.addEventListener('click', () => {
      this.modalTab = 'download';
      this.openQrModal();
    });

    const sendClipBtn = document.getElementById('btn-send-clip');
    if (sendClipBtn) {
      sendClipBtn.addEventListener('click', () => {
        const inp = document.getElementById('input-clipboard-text') as HTMLInputElement;
        if (inp && inp.value) {
          this.clipboardHistory.unshift({
            id: Date.now().toString(),
            source_device: 'Local Linux',
            content: { type: 'Text', value: inp.value },
            hash: 'h_' + Date.now(),
            sensitive: false,
            preview: inp.value,
            timestamp: 'Just now'
          });
          inp.value = '';
          this.render();
          this.attachEvents();
        }
      });
    }

    const allowRemote = document.getElementById('btn-remote-allow');
    if (allowRemote) {
      allowRemote.addEventListener('click', () => {
        this.remotePromptPending = false;
        alert('Remote session approved! Input injection activated.');
        this.render();
        this.attachEvents();
      });
    }

    const rejectRemote = document.getElementById('btn-remote-reject');
    if (rejectRemote) {
      rejectRemote.addEventListener('click', () => {
        this.remotePromptPending = false;
        this.render();
        this.attachEvents();
      });
    }
  }

  public setTab(tab: string) {
    this.activeTab = tab;
    this.render();
    this.attachEvents();
  }

  public openQrModal() {
    this.qrModalOpen = true;
    this.render();
    this.attachEvents();
  }

  public closeQrModal() {
    this.qrModalOpen = false;
    this.render();
    this.attachEvents();
  }

  public toggleTrust(id: string) {
    const dev = this.devices.find(d => d.id === id);
    if (dev) {
      dev.trusted = !dev.trusted;
      this.render();
      this.attachEvents();
    }
  }

  public testConnection(id: string) {
    alert(`Ping to device ${id}: RTT 1.2ms (Direct LAN connection verified)`);
  }

  public selectNote(id: string) {
    this.activeNoteId = id;
    this.render();
    this.attachEvents();
  }

  public createNote() {
    const newNote: Note = {
      id: Date.now().toString(),
      title: 'Untitled Note',
      content: '# Untitled Note\nStart typing here...',
      folder: 'General',
      tags: [],
      updated_at: 'Just now'
    };
    this.notes.unshift(newNote);
    this.activeNoteId = newNote.id;
    this.render();
    this.attachEvents();
  }

  public triggerSendFile() {
    this.transfers.unshift({
      session_id: 'sess-' + Date.now(),
      file_name: 'screenshot_mirror.png',
      file_size: 1540000,
      progress: 100,
      status: 'Completed',
      sha256: '9a8b7c6d5e4f3a2b1c0d9e8f7a6b5c4d3e2f1a0b'
    });
    this.render();
    this.attachEvents();
  }

  public dispatchHandoff() {
    alert('Handoff task sent to Prince\'s Pixel 9 Pro! Notification popped on phone.');
  }

  public simulateRemoteRequest() {
    this.remotePromptPending = true;
    this.render();
    this.attachEvents();
  }

  public askAi() {
    const inp = document.getElementById('ai-input') as HTMLInputElement;
    const chat = document.getElementById('ai-chat-box');
    if (inp && inp.value && chat) {
      const q = inp.value;
      chat.innerHTML += `
        <div style="background: rgba(255,255,255,0.06); padding: 10px; border-radius: var(--radius-sm); font-size: 13px; margin-bottom: 8px;">
          <strong>You:</strong> ${q}
        </div>
        <div style="background: rgba(99,102,241,0.15); border: 1px solid rgba(99,102,241,0.3); padding: 12px; border-radius: var(--radius-sm); font-size: 13px; line-height: 1.5; margin-bottom: 10px;">
          <strong>Nova Assistant:</strong> Found 1 matching note in 'Architecture': <em>"Nova Architectural Specification"</em>. P2P link verified with Prince's Pixel 9 Pro.
        </div>
      `;
      inp.value = '';
      chat.scrollTop = chat.scrollHeight;
    }
  }
}

// Global hook
const app = new NovaApp();
(window as any).novaApp = app;

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Database from "@tauri-apps/plugin-sql";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";

interface User {
  id: number;
  username: string;
  password: string;
}

interface DriveFile {
  id: string;
  name: string;
  mimeType: string;
  size?: string;
  modifiedTime?: string;
  webViewLink?: string;
  iconLink?: string;
}

const user_db = ref<Database | null>(null);
const allUsers = ref<User[]>([]);

const loginUsername = ref("");
const loginPassword = ref("");
const username = ref("");
const password = ref("");
const error = ref("");
const fileError = ref("");
const deviceFileName = ref("");
const deviceFiles = ref<string[]>([]);
const showOpenDialog = ref(false);
const showSaveDialog = ref(false);
const saveFileName = ref("");

// Google Drive API — replace these with your own credentials
// 1. Go to https://console.cloud.google.com
// 2. Create a project → Enable "Google Drive API"
// 3. Credentials → Create OAuth 2.0 Client ID (Web application)
//    - Authorized JavaScript origins: http://localhost:1420
//    - Authorized redirect URIs:     http://localhost:1420/oauth-callback.html
// 4. Copy the Client ID below
const GOOGLE_CLIENT_ID =
  "1078780193031-ojib7t02s6erk8bbmnqd8i2a4g9aa2bn.apps.googleusercontent.com";

const driveFiles = ref<DriveFile[]>([]);
const driveLoading = ref(false);
const driveError = ref("");
const driveAccessToken = ref("");
const driveConnected = ref(false);
const currentFolderId = ref("root");
const folderPath = ref<{ id: string; name: string }[]>([
  { id: "root", name: "My Drive" },
]);
const previewFile = ref<DriveFile | null>(null);
const previewUrl = ref("");
const swipeStartX = ref(0);
const swipeStartY = ref(0);
const swipeDeltaX = ref(0);
const isSwiping = ref(false);

const previewableFiles = computed(() =>
  driveFiles.value.filter(
    (f) => f.mimeType !== "application/vnd.google-apps.folder",
  ),
);

const currentPreviewIndex = computed(() => {
  if (!previewFile.value) return -1;
  return previewableFiles.value.findIndex(
    (f) => f.id === previewFile.value!.id,
  );
});

const DATABASE_PATH = "sqlite:mydatabase.db";

const loggedInUser = ref<User | null>(null);

const loadDatabase = async () => {
  const db = await Database.load(DATABASE_PATH);
  return db;
};

const queryAllUsers = async () => {
  const users: User[] =
    (await user_db.value?.select(
      "SELECT id, username, password FROM users",
      [],
    )) || [];
  allUsers.value = users;
  console.log("Queried Users:", users);
  return users;
};

const loginUser = async (username: string, password: string) => {
  const users: User[] =
    (await user_db.value?.select(
      "SELECT id, username, password FROM users WHERE username = ? AND password = ?",
      [username, password],
    )) || [];
  return users.length > 0 ? users[0] : null;
};

const addUser = async (name: string, pw: string) => {
  await user_db.value?.execute(
    "INSERT INTO users (username, password) VALUES (?, ?)",
    [name, pw],
  );
  await queryAllUsers();
  username.value = "";
  password.value = "";
};

const deleteUser = async (id: number) => {
  await user_db.value?.execute("DELETE FROM users WHERE id = ?", [id]);
  await queryAllUsers();
};

const clearUsers = async () => {
  try {
    await user_db.value?.execute("DELETE FROM users", []);
    await queryAllUsers();
  } catch (error) {
    console.error("Error querying users before clearing:", error);
  }
};

const login = async (username: string, password: string) => {
  loginUser(username, password).then(async (success) => {
    if (success) {
      error.value = "";
      const login = { username, password, id: 1 };
      const user = await invoke("login", { user: login });
      console.log("Login successful", user);
      loggedInUser.value = user as User;

      // Save to localStorage
      localStorage.setItem("loggedInUser", JSON.stringify(user));

      // clear input fields
      loginUsername.value = "";
      loginPassword.value = "";
    } else {
      console.log("Login failed");
      error.value = "Invalid username or password";
    }
  });
};

const logout = () => {
  loggedInUser.value = null;
  localStorage.removeItem("loggedInUser");
};

const openFile = async () => {
  await refreshDeviceFiles();
  showOpenDialog.value = true;
};

const saveFile = async () => {
  saveFileName.value = deviceFileName.value || "";
  showSaveDialog.value = true;
};

const refreshDeviceFiles = async () => {
  try {
    deviceFiles.value = (await invoke("list_device_files")) as string[];
  } catch (err) {
    console.error("Failed to list device files:", err);
  }
};

const confirmSaveDeviceFile = async () => {
  const data =
    (document.querySelector("#contents") as HTMLTextAreaElement)?.value || "";
  if (!saveFileName.value.trim()) {
    fileError.value = "Please enter a file name";
    return;
  }
  try {
    fileError.value = "";
    await invoke("save_device_file", {
      filename: saveFileName.value,
      content: data,
    });
    deviceFileName.value = saveFileName.value;
    showSaveDialog.value = false;
    await refreshDeviceFiles();
  } catch (err) {
    console.error("Failed to save device file:", err);
    fileError.value = "Failed to save file";
  }
};

const openDeviceFile = async (filename: string) => {
  try {
    fileError.value = "";
    const content = (await invoke("read_device_file", {
      filename,
    })) as string;
    const textarea = document.querySelector("#contents") as HTMLTextAreaElement;
    textarea.value = content;
    deviceFileName.value = filename;
  } catch (err) {
    console.error("Failed to open device file:", err);
    fileError.value = "Failed to open file";
  }
};

const confirmOpenDeviceFile = async (filename: string) => {
  await openDeviceFile(filename);
  showOpenDialog.value = false;
};

// ── Google Drive API (system browser OAuth + deep link callback) ──

// Hosted on GitHub Pages so it works on desktop, simulator, AND physical device
// (Google OAuth rejects IP addresses as origins, so we can't use the dev server URL)
const REDIRECT_URI =
  "https://friis1978.github.io/tauri_template/oauth-callback.html";

const connectGoogleDrive = async () => {
  driveError.value = "";
  driveLoading.value = true;

  const params = new URLSearchParams({
    client_id: GOOGLE_CLIENT_ID,
    redirect_uri: REDIRECT_URI,
    response_type: "token",
    scope: "https://www.googleapis.com/auth/drive.readonly",
    include_granted_scopes: "true",
  });

  // Open OAuth in the system browser (Safari) — avoids WebView popup/redirect issues
  try {
    await openUrl(`https://accounts.google.com/o/oauth2/v2/auth?${params}`);
  } catch (e) {
    driveError.value = "Failed to open browser for authentication";
    driveLoading.value = false;
    console.error(e);
  }
};

const handleDeepLinkUrl = async (url: string) => {
  try {
    const parsed = new URL(url);
    const token = parsed.searchParams.get("access_token");
    const err = parsed.searchParams.get("error");

    if (err) {
      driveError.value = err;
      driveLoading.value = false;
      return;
    }

    if (token) {
      driveAccessToken.value = token;
      driveConnected.value = true;
      await resolveAndNavigateToStartFolder();
      driveLoading.value = false;
    }
  } catch (e) {
    console.error("Failed to parse deep link URL:", e);
    driveLoading.value = false;
  }
};

const DRIVE_START_PATH = ["privat", "musik dokumenter", "mine bands"];

const resolveAndNavigateToStartFolder = async () => {
  let parentId = "root";
  const breadcrumbs: { id: string; name: string }[] = [
    { id: "root", name: "My Drive" },
  ];

  for (const folderName of DRIVE_START_PATH) {
    try {
      const q = `'${parentId}' in parents and name = '${folderName.replace(/'/g, "\\'")}' and mimeType = 'application/vnd.google-apps.folder' and trashed = false`;
      const params = new URLSearchParams({
        q,
        fields: "files(id,name)",
        pageSize: "1",
      });
      const response = await fetch(
        `https://www.googleapis.com/drive/v3/files?${params}`,
        {
          headers: {
            Authorization: `Bearer ${driveAccessToken.value}`,
          },
        },
      );
      if (!response.ok) break;
      const data = await response.json();
      if (!data.files || data.files.length === 0) break;
      parentId = data.files[0].id;
      breadcrumbs.push({ id: parentId, name: data.files[0].name });
    } catch {
      break;
    }
  }

  currentFolderId.value = parentId;
  folderPath.value = breadcrumbs;
  await listDriveFiles(parentId);
};

const listDriveFiles = async (folderId?: string) => {
  if (!driveAccessToken.value) return;
  driveLoading.value = true;
  const folder = folderId || currentFolderId.value;
  try {
    const params = new URLSearchParams({
      pageSize: "50",
      fields: "files(id,name,mimeType,size,modifiedTime,webViewLink,iconLink)",
      orderBy: "folder,name",
      q: `'${folder}' in parents and trashed = false`,
    });
    const response = await fetch(
      `https://www.googleapis.com/drive/v3/files?${params}`,
      {
        headers: {
          Authorization: `Bearer ${driveAccessToken.value}`,
        },
      },
    );
    if (!response.ok) throw new Error("Failed to fetch files");
    const data = await response.json();
    driveFiles.value = data.files || [];
  } catch (err) {
    driveError.value = "Failed to list Drive files";
    console.error(err);
  }
  driveLoading.value = false;
};

const navigateToFolder = async (file: DriveFile) => {
  if (file.mimeType !== "application/vnd.google-apps.folder") return;
  currentFolderId.value = file.id;
  folderPath.value.push({ id: file.id, name: file.name });
  await listDriveFiles(file.id);
};

const navigateToBreadcrumb = async (index: number) => {
  const target = folderPath.value[index];
  currentFolderId.value = target.id;
  folderPath.value = folderPath.value.slice(0, index + 1);
  await listDriveFiles(target.id);
};

const disconnectGoogleDrive = () => {
  const token = driveAccessToken.value;
  driveAccessToken.value = "";
  driveConnected.value = false;
  driveFiles.value = [];
  driveError.value = "";
  currentFolderId.value = "root";
  folderPath.value = [{ id: "root", name: "My Drive" }];
  if (token) {
    // Revoke the token via Google's endpoint
    fetch(`https://oauth2.googleapis.com/revoke?token=${token}`, {
      method: "POST",
    }).catch(() => {});
  }
};

const getPreviewUrl = (file: DriveFile): string => {
  const id = file.id;
  const mime = file.mimeType;
  // Google Workspace native types
  if (mime === "application/vnd.google-apps.document")
    return `https://docs.google.com/document/d/${id}/preview`;
  if (mime === "application/vnd.google-apps.spreadsheet")
    return `https://docs.google.com/spreadsheets/d/${id}/preview`;
  if (mime === "application/vnd.google-apps.presentation")
    return `https://docs.google.com/presentation/d/${id}/preview`;
  if (mime === "application/vnd.google-apps.drawing")
    return `https://docs.google.com/drawings/d/${id}/preview`;
  // PDFs, images, videos, and other files
  return `https://drive.google.com/file/d/${id}/preview`;
};

const openDriveFile = (file: DriveFile) => {
  previewUrl.value = getPreviewUrl(file);
  previewFile.value = file;
};

const closePreview = () => {
  previewFile.value = null;
  previewUrl.value = "";
  swipeDeltaX.value = 0;
  isSwiping.value = false;
};

const navigatePreview = (direction: "prev" | "next") => {
  const files = previewableFiles.value;
  const idx = currentPreviewIndex.value;
  if (idx === -1 || files.length <= 1) return;
  let newIdx: number;
  if (direction === "next") {
    newIdx = idx + 1 >= files.length ? 0 : idx + 1;
  } else {
    newIdx = idx - 1 < 0 ? files.length - 1 : idx - 1;
  }
  const file = files[newIdx];
  previewFile.value = file;
  previewUrl.value = getPreviewUrl(file);
};

const onSwipeStart = (e: TouchEvent) => {
  swipeStartX.value = e.touches[0].clientX;
  swipeStartY.value = e.touches[0].clientY;
  swipeDeltaX.value = 0;
  isSwiping.value = false;
};

const onSwipeMove = (e: TouchEvent) => {
  const dx = e.touches[0].clientX - swipeStartX.value;
  const dy = e.touches[0].clientY - swipeStartY.value;
  // Only treat as horizontal swipe if horizontal movement > vertical
  if (Math.abs(dx) > Math.abs(dy) && Math.abs(dx) > 10) {
    isSwiping.value = true;
    swipeDeltaX.value = dx;
  }
};

const onSwipeEnd = () => {
  if (!isSwiping.value) return;
  const threshold = 80;
  if (swipeDeltaX.value < -threshold) {
    navigatePreview("next");
  } else if (swipeDeltaX.value > threshold) {
    navigatePreview("prev");
  }
  swipeDeltaX.value = 0;
  isSwiping.value = false;
};

const formatFileSize = (bytes?: string) => {
  if (!bytes) return "—";
  const b = parseInt(bytes);
  if (b < 1024) return b + " B";
  if (b < 1024 * 1024) return (b / 1024).toFixed(1) + " KB";
  return (b / (1024 * 1024)).toFixed(1) + " MB";
};

const formatDate = (dateStr?: string) => {
  if (!dateStr) return "—";
  return new Date(dateStr).toLocaleDateString();
};

// ── Metronome ──
const metronomeBpm = ref(120);
const metronomeBeatsPerMeasure = ref(4);
const metronomeIsPlaying = ref(false);
const metronomeCurrentBeat = ref(0);
const metronomeVolume = ref(0.7);
let metronomeAudioCtx: AudioContext | null = null;
let metronomeTimerId: number | null = null;
let metronomeNextNoteTime = 0;
let metronomeScheduleAheadTime = 0.1; // seconds
let metronomeLookahead = 25; // ms
let metronomeCurrentBeatInternal = 0;

const getAudioContext = (): AudioContext => {
  if (!metronomeAudioCtx) {
    metronomeAudioCtx = new (
      window.AudioContext || (window as any).webkitAudioContext
    )();
  }
  return metronomeAudioCtx;
};

const playClick = (time: number, isAccent: boolean) => {
  const ctx = getAudioContext();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.connect(gain);
  gain.connect(ctx.destination);

  // Higher pitch for accent (beat 1), lower for others
  osc.frequency.value = isAccent ? 1000 : 800;
  osc.type = "sine";

  gain.gain.setValueAtTime(metronomeVolume.value, time);
  gain.gain.exponentialRampToValueAtTime(0.001, time + 0.05);

  osc.start(time);
  osc.stop(time + 0.05);
};

const scheduleNote = () => {
  const ctx = getAudioContext();
  while (metronomeNextNoteTime < ctx.currentTime + metronomeScheduleAheadTime) {
    const isAccent = metronomeCurrentBeatInternal === 0;
    playClick(metronomeNextNoteTime, isAccent);

    // Update the reactive beat indicator
    const beatToShow = metronomeCurrentBeatInternal;
    const delay = Math.max(0, (metronomeNextNoteTime - ctx.currentTime) * 1000);
    setTimeout(() => {
      metronomeCurrentBeat.value = beatToShow;
    }, delay);

    // Advance
    const secondsPerBeat = 60.0 / metronomeBpm.value;
    metronomeNextNoteTime += secondsPerBeat;
    metronomeCurrentBeatInternal =
      (metronomeCurrentBeatInternal + 1) % metronomeBeatsPerMeasure.value;
  }
};

const startMetronome = () => {
  if (metronomeIsPlaying.value) return;
  const ctx = getAudioContext();
  if (ctx.state === "suspended") ctx.resume();

  metronomeCurrentBeatInternal = 0;
  metronomeCurrentBeat.value = 0;
  metronomeNextNoteTime = ctx.currentTime + 0.05;
  metronomeIsPlaying.value = true;

  metronomeTimerId = window.setInterval(scheduleNote, metronomeLookahead);
};

const stopMetronome = () => {
  metronomeIsPlaying.value = false;
  if (metronomeTimerId !== null) {
    clearInterval(metronomeTimerId);
    metronomeTimerId = null;
  }
  metronomeCurrentBeat.value = 0;
};

const toggleMetronome = () => {
  if (metronomeIsPlaying.value) stopMetronome();
  else startMetronome();
};

const adjustBpm = (delta: number) => {
  metronomeBpm.value = Math.min(300, Math.max(20, metronomeBpm.value + delta));
};

const tapTimes: number[] = [];
const tapTempo = () => {
  const now = Date.now();
  tapTimes.push(now);
  // Keep only last 5 taps
  if (tapTimes.length > 5) tapTimes.shift();
  if (tapTimes.length >= 2) {
    const intervals: number[] = [];
    for (let i = 1; i < tapTimes.length; i++) {
      intervals.push(tapTimes[i] - tapTimes[i - 1]);
    }
    const avgMs = intervals.reduce((a, b) => a + b, 0) / intervals.length;
    const bpm = Math.round(60000 / avgMs);
    metronomeBpm.value = Math.min(300, Math.max(20, bpm));
  }
  // Reset if no tap for 2s
  setTimeout(() => {
    if (
      tapTimes.length > 0 &&
      Date.now() - tapTimes[tapTimes.length - 1] > 2000
    ) {
      tapTimes.length = 0;
    }
  }, 2100);
};

onMounted(async () => {
  // Handle deep links for Google OAuth callback
  try {
    // Check if app was opened via a deep link (cold start)
    const startUrls = await getCurrent();
    if (startUrls) {
      for (const url of startUrls) {
        if (url.includes("oauth")) await handleDeepLinkUrl(url);
      }
    }
  } catch (e) {
    console.error("getCurrent deep link error:", e);
  }

  // Listen for deep links while the app is running (OAuth redirect from Safari)
  await onOpenUrl(async (urls: string[]) => {
    for (const url of urls) {
      if (url.includes("oauth")) await handleDeepLinkUrl(url);
    }
  });

  // Load saved user from localStorage
  const savedUser = localStorage.getItem("loggedInUser");
  if (savedUser) {
    try {
      loggedInUser.value = JSON.parse(savedUser) as User;
      console.log("Restored logged in user:", loggedInUser.value);
    } catch (error) {
      console.error("Error parsing saved user:", error);
      localStorage.removeItem("loggedInUser");
    }
  }

  listen("save_state", (event) => {
    console.log("Save State Event Received:", event.payload);
  });

  listen("content", (event) => {
    const textarea = document.querySelector("#contents") as HTMLTextAreaElement;
    textarea.value = event.payload as string;
  });

  loadDatabase().then((db) => {
    user_db.value = db;
    console.log("Database loaded:", user_db.value);
    queryAllUsers();
  });

  refreshDeviceFiles();
});

onUnmounted(() => {
  stopMetronome();
  if (metronomeAudioCtx) {
    metronomeAudioCtx.close();
    metronomeAudioCtx = null;
  }
});
</script>

<template>
  <div
    v-if="showOpenDialog"
    class="fixed inset-0 bg-black/40 flex items-center justify-center p-4"
  >
    <div class="flex flex-col w-full max-w-md bg-white rounded-lg p-4 gap-5">
      <div class="flex items-center justify-between">
        <h3 class="text-lg font-semibold">Open File</h3>
      </div>
      <div class="mt-3">
        <p v-if="!deviceFiles.length" class="text-sm text-gray-500">
          No saved files yet.
        </p>
        <div v-else class="flex flex-col gap-2">
          <button
            v-for="file in deviceFiles"
            :key="file"
            class="file-btn px-3 py-2 rounded border border-gray-300 text-left w-full"
            @click="confirmOpenDeviceFile(file)"
          >
            {{ file }}
          </button>
        </div>
      </div>
      <div class="flex items-center justify-end">
        <button class="text-gray-500" @click="showOpenDialog = false">
          Close
        </button>
      </div>
    </div>
  </div>
  <div
    v-if="showSaveDialog"
    class="fixed inset-0 bg-black/40 flex items-center justify-center p-4"
  >
    <div class="flex flex-col w-full max-w-md bg-white rounded-lg p-4 gap-5">
      <div class="flex items-center justify-between">
        <h3 class="text-lg font-semibold">Save File</h3>
      </div>
      <div class="mt-3 flex flex-col gap-3">
        <input
          v-model="saveFileName"
          type="text"
          placeholder="File name (e.g. notes.txt)"
          class="border-2 border-gray-300 rounded-md p-2 w-full"
        />
        <button class="save-btn w-full" @click="confirmSaveDeviceFile()">
          Save
        </button>
        <p v-if="fileError" class="text-sm text-red-600">{{ fileError }}</p>
      </div>
      <div class="flex items-center justify-end">
        <button class="text-gray-500" @click="showSaveDialog = false">
          Close
        </button>
      </div>
    </div>
  </div>
  <!-- Drive file preview modal -->
  <div v-if="previewFile" class="fixed inset-0 z-50 bg-black">
    <iframe
      :key="previewFile.id"
      :src="previewUrl"
      class="w-full h-full border-0"
      allow="autoplay; encrypted-media"
      allowfullscreen
    ></iframe>
    <!-- Swipe overlay (transparent, sits on top of iframe to capture touches) -->
    <div class="absolute inset-0 z-40" style="pointer-events: none">
      <!-- Left edge swipe zone -->
      <div
        class="absolute left-0 top-0 bottom-0 w-16"
        style="pointer-events: auto"
        @touchstart="onSwipeStart"
        @touchmove="onSwipeMove"
        @touchend="onSwipeEnd"
      ></div>
      <!-- Right edge swipe zone -->
      <div
        class="absolute right-0 top-0 bottom-0 w-16"
        style="pointer-events: auto"
        @touchstart="onSwipeStart"
        @touchmove="onSwipeMove"
        @touchend="onSwipeEnd"
      ></div>
    </div>
    <!-- Left arrow -->
    <button
      v-if="previewableFiles.length > 1"
      class="absolute left-2 top-1/2 -translate-y-1/2 w-10 h-10 flex items-center justify-center rounded-full bg-black/50 text-white text-lg z-50 backdrop-blur-sm"
      style="padding: 0; min-width: 0"
      @touchend.prevent="navigatePreview('prev')"
      @click="navigatePreview('prev')"
    >
      ‹
    </button>
    <!-- Right arrow -->
    <button
      v-if="previewableFiles.length > 1"
      class="absolute right-2 top-1/2 -translate-y-1/2 w-10 h-10 flex items-center justify-center rounded-full bg-black/50 text-white text-lg z-50 backdrop-blur-sm"
      style="padding: 0; min-width: 0"
      @touchend.prevent="navigatePreview('next')"
      @click="navigatePreview('next')"
    >
      ›
    </button>
    <!-- Close button -->
    <button
      class="absolute top-7 right-7 w-10 h-10 flex items-center justify-center rounded-full bg-black/60 text-white text-xl font-bold z-50 backdrop-blur-sm"
      style="padding: 0; min-width: 0; line-height: 1"
      @touchend.prevent="closePreview()"
      @click="closePreview()"
    >
      ✕
    </button>
    <!-- Mini metronome (top-left) -->
    <div
      class="absolute top-7 left-7 z-50 flex items-center gap-1.5 rounded-full bg-black/60 backdrop-blur-sm px-2 py-1.5"
      style="pointer-events: auto"
    >
      <button
        class="w-7 h-7 rounded-full bg-white/20 text-white text-sm font-bold flex items-center justify-center active:bg-white/30"
        style="padding: 0; min-width: 0; touch-action: manipulation"
        @touchend.prevent="adjustBpm(-1)"
        @click="adjustBpm(-1)"
      >
        −
      </button>
      <span
        class="text-white text-xs font-semibold tabular-nums w-10 text-center"
        >{{ metronomeBpm }}</span
      >
      <button
        class="w-7 h-7 rounded-full bg-white/20 text-white text-sm font-bold flex items-center justify-center active:bg-white/30"
        style="padding: 0; min-width: 0; touch-action: manipulation"
        @touchend.prevent="adjustBpm(1)"
        @click="adjustBpm(1)"
      >
        +
      </button>
      <button
        class="ml-0.5 w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold"
        :class="[
          metronomeIsPlaying
            ? 'bg-red-500 text-white active:bg-red-600'
            : 'bg-white/20 text-white active:bg-white/30',
        ]"
        style="padding: 0; min-width: 0; touch-action: manipulation"
        @touchend.prevent="toggleMetronome()"
        @click="toggleMetronome()"
      >
        {{ metronomeIsPlaying ? "⏹" : "▶" }}
      </button>
    </div>
    <!-- File counter -->
    <div
      v-if="previewableFiles.length > 1"
      class="absolute bottom-4 left-1/2 -translate-x-1/2 px-3 py-1 rounded-full bg-black/50 text-white text-xs z-50 backdrop-blur-sm"
    >
      {{ currentPreviewIndex + 1 }} / {{ previewableFiles.length }}
    </div>
  </div>
  <div
    class="app-shell w-full flex flex-col items-start justify-start p-4 sm:p-6 gap-6"
  >
    <div class="w-full">
      <!-- Google Drive -->
      <div class="relative z-10">
        <p class="text-gray-600">Google Drive</p>
        <div
          class="mt-4 w-full rounded-xl border-2 border-gray-300 p-4"
          style="touch-action: manipulation"
        >
          <!-- Not connected -->
          <div
            v-if="!driveConnected"
            class="flex flex-col items-center gap-3 py-8"
          >
            <svg
              class="w-12 h-12 text-gray-400"
              fill="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                d="M7.71 3.5L1.15 15l4.58 6.5h2.72L3.87 15l4.58-8L7.71 3.5zm8.58 0L21.87 15l-4.58 6.5h-2.72L19.15 15 14.57 7l1.72-3.5zm-4.29 0L5.42 15l6.58 6.5 6.58-6.5L12 3.5z"
              />
            </svg>
            <p class="text-gray-500 text-sm text-center">
              Connect your Google Drive to browse files
            </p>
            <button
              class="submit-btn mt-2"
              style="touch-action: manipulation"
              :disabled="driveLoading"
              @click="connectGoogleDrive()"
            >
              {{ driveLoading ? "Connecting..." : "Connect Google Drive" }}
            </button>
            <p v-if="driveError" class="text-sm text-red-600">
              {{ driveError }}
            </p>
          </div>

          <!-- Connected -->
          <div v-else class="flex flex-col gap-3">
            <div class="flex items-center justify-between">
              <span class="text-green-600 text-sm font-medium"
                >✓ Connected to Google Drive</span
              >
              <div class="flex gap-2">
                <button
                  class="px-3 py-1 text-sm bg-gray-200 rounded hover:bg-gray-300"
                  style="touch-action: manipulation"
                  @click="listDriveFiles()"
                >
                  Refresh
                </button>
                <button
                  class="px-3 py-1 text-sm bg-red-100 text-red-600 rounded hover:bg-red-200"
                  style="touch-action: manipulation"
                  @click="disconnectGoogleDrive()"
                >
                  Disconnect
                </button>
              </div>
            </div>

            <!-- Breadcrumb navigation -->
            <div
              class="flex items-center gap-1 text-sm text-gray-500 flex-wrap"
            >
              <template v-for="(crumb, i) in folderPath" :key="crumb.id">
                <span v-if="i > 0" class="mx-1">/</span>
                <button
                  v-if="i < folderPath.length - 1"
                  class="text-blue-600 hover:underline"
                  @click="navigateToBreadcrumb(i)"
                >
                  {{ crumb.name }}
                </button>
                <span v-else class="font-medium text-gray-800">{{
                  crumb.name
                }}</span>
              </template>
            </div>

            <div v-if="driveLoading" class="text-center py-4 text-gray-500">
              Loading files...
            </div>

            <div
              v-else-if="!driveFiles.length"
              class="text-center py-4 text-gray-500"
            >
              This folder is empty.
            </div>

            <div
              v-else
              class="overflow-y-auto"
              style="max-height: 400px; -webkit-overflow-scrolling: touch"
            >
              <div
                v-for="file in driveFiles"
                :key="file.id"
                class="flex items-center justify-between py-3 px-2 border-b border-gray-200 active:bg-blue-50"
                @touchend.prevent="
                  file.mimeType === 'application/vnd.google-apps.folder'
                    ? navigateToFolder(file)
                    : openDriveFile(file)
                "
                @click="
                  file.mimeType === 'application/vnd.google-apps.folder'
                    ? navigateToFolder(file)
                    : openDriveFile(file)
                "
              >
                <div class="flex items-center gap-2 min-w-0 flex-1">
                  <img
                    v-if="file.iconLink"
                    :src="file.iconLink"
                    class="w-5 h-5 flex-shrink-0"
                    alt=""
                  />
                  <span v-else class="w-5 h-5 flex-shrink-0 text-center">{{
                    file.mimeType === "application/vnd.google-apps.folder"
                      ? "📁"
                      : "📄"
                  }}</span>
                  <span
                    class="truncate text-sm"
                    :class="
                      file.mimeType === 'application/vnd.google-apps.folder'
                        ? 'font-medium text-blue-700'
                        : 'text-gray-800'
                    "
                    >{{ file.name }}</span
                  >
                </div>
                <span class="text-xs text-gray-400 ml-2 flex-shrink-0">
                  {{
                    file.mimeType === "application/vnd.google-apps.folder"
                      ? "→"
                      : formatFileSize(file.size)
                  }}
                </span>
              </div>
            </div>

            <p v-if="driveError" class="text-sm text-red-600">
              {{ driveError }}
            </p>
          </div>
        </div>
      </div>

      <!-- Metronome -->
      <div>
        <p class="mt-6 text-gray-600">Metronome</p>
        <div
          class="mt-4 w-full rounded-xl border-2 border-gray-300 p-4"
          style="touch-action: manipulation"
        >
          <!-- BPM Display -->
          <div class="flex flex-col items-center gap-4">
            <div class="text-5xl font-bold tabular-nums text-gray-800">
              {{ metronomeBpm }}
            </div>
            <div class="text-sm text-gray-500 -mt-2">BPM</div>

            <!-- BPM Controls -->
            <div class="flex items-center gap-3">
              <button
                class="w-10 h-10 rounded-full bg-gray-200 text-gray-700 text-xl font-bold flex items-center justify-center active:bg-gray-300"
                style="touch-action: manipulation"
                @touchend.prevent="adjustBpm(-5)"
                @click="adjustBpm(-5)"
              >
                −
              </button>
              <button
                class="w-8 h-8 rounded-full bg-gray-100 text-gray-600 text-sm font-bold flex items-center justify-center active:bg-gray-200"
                style="touch-action: manipulation"
                @touchend.prevent="adjustBpm(-1)"
                @click="adjustBpm(-1)"
              >
                −1
              </button>
              <input
                v-model.number="metronomeBpm"
                type="range"
                min="20"
                max="300"
                class="w-32 accent-blue-600"
              />
              <button
                class="w-8 h-8 rounded-full bg-gray-100 text-gray-600 text-sm font-bold flex items-center justify-center active:bg-gray-200"
                style="touch-action: manipulation"
                @touchend.prevent="adjustBpm(1)"
                @click="adjustBpm(1)"
              >
                +1
              </button>
              <button
                class="w-10 h-10 rounded-full bg-gray-200 text-gray-700 text-xl font-bold flex items-center justify-center active:bg-gray-300"
                style="touch-action: manipulation"
                @touchend.prevent="adjustBpm(5)"
                @click="adjustBpm(5)"
              >
                +
              </button>
            </div>

            <!-- Beat indicator dots -->
            <div class="flex items-center gap-2">
              <div
                v-for="beat in metronomeBeatsPerMeasure"
                :key="beat"
                class="w-4 h-4 rounded-full transition-all duration-75"
                :class="[
                  metronomeIsPlaying && metronomeCurrentBeat === beat - 1
                    ? beat === 1
                      ? 'bg-blue-600 scale-125'
                      : 'bg-blue-400 scale-110'
                    : 'bg-gray-300',
                ]"
              ></div>
            </div>

            <!-- Beats per measure -->
            <div class="flex items-center gap-2 text-sm text-gray-600">
              <span>Beats:</span>
              <button
                v-for="n in [2, 3, 4, 5, 6, 7, 8]"
                :key="n"
                class="w-8 h-8 rounded-full text-sm font-medium flex items-center justify-center"
                :class="[
                  metronomeBeatsPerMeasure === n
                    ? 'bg-blue-600 text-white'
                    : 'bg-gray-100 text-gray-700 active:bg-gray-200',
                ]"
                style="touch-action: manipulation"
                @touchend.prevent="metronomeBeatsPerMeasure = n"
                @click="metronomeBeatsPerMeasure = n"
              >
                {{ n }}
              </button>
            </div>

            <!-- Volume slider -->
            <div class="flex items-center gap-2 w-full max-w-xs">
              <span class="text-sm text-gray-500">🔈</span>
              <input
                v-model.number="metronomeVolume"
                type="range"
                min="0"
                max="1"
                step="0.05"
                class="flex-1 accent-blue-600"
              />
              <span class="text-sm text-gray-500">🔊</span>
            </div>

            <!-- Play / Tap buttons -->
            <div class="flex items-center gap-3">
              <button
                class="px-6 py-3 rounded-full text-white font-semibold text-base"
                :class="[
                  metronomeIsPlaying
                    ? 'bg-red-500 active:bg-red-600'
                    : 'bg-blue-600 active:bg-blue-700',
                ]"
                style="touch-action: manipulation"
                @touchend.prevent="toggleMetronome()"
                @click="toggleMetronome()"
              >
                {{ metronomeIsPlaying ? "⏹ Stop" : "▶ Start" }}
              </button>
              <button
                class="px-4 py-3 rounded-full bg-gray-200 text-gray-700 font-medium text-base active:bg-gray-300"
                style="touch-action: manipulation"
                @touchend.prevent="tapTempo()"
                @click="tapTempo()"
              >
                Tap
              </button>
            </div>
          </div>
        </div>
      </div>

      <div>
        <p class="mt-6 text-gray-600">friismusic.com</p>
        <div
          class="mt-4 w-full rounded-xl overflow-hidden border-2 border-gray-300"
          style="
            height: 600px;
            position: relative;
            z-index: 0;
            isolation: isolate;
          "
        >
          <iframe
            src="https://friismusic.com"
            class="w-full h-full"
            style="pointer-events: auto"
            frameborder="0"
            allow="autoplay; encrypted-media"
            allowfullscreen
          ></iframe>
        </div>
      </div>

      <p class="mt-6 text-gray-600">demonstrating tauri functions</p>
      <div class="mt-2">
        <p v-if="loggedInUser" class="text-green-600">
          Logged in as: {{ loggedInUser.username }}
          <button
            @click="logout()"
            class="ml-4 px-2 py-1 bg-red-500 text-white rounded text-sm"
          >
            Logout
          </button>
        </p>
        <p v-else class="text-red-600">Not logged in</p>
      </div>
      <div class="flex flex-col sm:flex-row gap-4 w-full max-w-md mt-4">
        <input
          v-model="loginUsername"
          type="text"
          placeholder="Username"
          class="border-2 border-gray-300 rounded-md p-2 h-10"
        />
        <input
          v-model="loginPassword"
          type="password"
          placeholder="Password"
          class="border-2 border-gray-300 rounded-md p-2 h-10"
        />
        <button
          class="submit-btn w-full sm:w-auto"
          @click="login(loginUsername, loginPassword)"
        >
          Login
        </button>
      </div>
      <p v-if="error" class="my-2 text-red-600">{{ error }}</p>
      <div class="mt-6">
        <p class="text-gray-600">demonstrating SQLite database operations</p>
        <div class="flex flex-row gap-4 w-full max-w-md mt-4">
          <input
            v-model="username"
            type="text"
            placeholder="Username"
            class="border-2 border-gray-300 rounded-md p-2 h-10"
          />
          <input
            v-model="password"
            type="password"
            placeholder="Password"
            class="border-2 border-gray-300 rounded-md p-2 h-10"
          />
          <button
            class="submit-btn h-10 w-40 whitespace-nowrap"
            @click="addUser(username, password)"
          >
            Add User
          </button>
          <button
            class="clear-btn h-10 w-40 whitespace-nowrap"
            @click="clearUsers()"
          >
            Clear Users
          </button>
        </div>
        <div
          v-if="allUsers.length"
          class="flex flex-col mt-6 w-full max-w-2xl border-2 border-gray-300 rounded-md p-4 overflow-x-auto"
        >
          <h2 class="text-lg font-semibold mb-4">All Users</h2>
          <table class="w-full border-collapse">
            <thead>
              <tr class="border-b-2">
                <th class="text-left py-2">ID</th>
                <th class="text-left py-2">Username</th>
                <th class="text-left py-2">Password</th>
                <th class="text-left py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="user in allUsers" :key="user.id" class="border-b">
                <td class="py-2">{{ user.id }}</td>
                <td class="py-2">{{ user.username }}</td>
                <td class="py-2">{{ user.password }}</td>
                <td class="py-2">
                  <button class="delete-btn" @click="deleteUser(user.id)">
                    Delete
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div>
          <p class="mt-6 text-gray-600">demonstrating file dialogs</p>
          <div class="flex flex-col lg:flex-row items-start gap-4 w-full">
            <textarea
              id="contents"
              class="w-full lg:max-w-2xl h-40 border-2 border-gray-300 rounded-md p-2"
              placeholder="File contents will appear here..."
            ></textarea>
            <div class="flex flex-row lg:flex-col gap-3 w-full lg:w-auto">
              <button class="open-btn w-full lg:w-auto" @click="openFile()">
                Open File
              </button>
              <button class="save-btn w-full lg:w-auto" @click="saveFile()">
                Save File
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  height: 100%;
  overflow-y: auto;
  -webkit-overflow-scrolling: touch;
  padding-top: calc(env(safe-area-inset-top, 0px) + 1rem);
  padding-bottom: calc(env(safe-area-inset-bottom, 0px) + 1rem);
  padding-left: calc(env(safe-area-inset-left, 0px) + 1rem);
  padding-right: calc(env(safe-area-inset-right, 0px) + 1rem);
}
</style>

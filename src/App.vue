<script setup lang="ts">
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Database from "@tauri-apps/plugin-sql";

interface User {
  id: number;
  username: string;
  password: string;
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

onMounted(() => {
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
  <div
    class="app-shell w-full flex flex-col items-start justify-start p-4 sm:p-6 gap-6"
  >
    <div class="w-full">
      <p class="text-gray-600">demonstrating tauri functions</p>
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
        <div>
          <p class="mt-6 text-gray-600">friismusic.com</p>
          <div
            class="mt-4 w-full rounded-xl overflow-hidden border-2 border-gray-300"
            style="height: 600px"
          >
            <iframe
              src="https://friismusic.com"
              class="w-full h-full"
              frameborder="0"
              allow="autoplay; encrypted-media"
              allowfullscreen
            ></iframe>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  padding-top: calc(env(safe-area-inset-top, 0px) + 1rem);
  padding-bottom: calc(env(safe-area-inset-bottom, 0px) + 1rem);
  padding-left: calc(env(safe-area-inset-left, 0px) + 1rem);
  padding-right: calc(env(safe-area-inset-right, 0px) + 1rem);
}
</style>

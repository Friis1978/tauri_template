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
      []
    )) || [];
  allUsers.value = users;
  console.log("Queried Users:", users);
  return users;
};

const loginUser = async (username: string, password: string) => {
  const users: User[] =
    (await user_db.value?.select(
      "SELECT id, username, password FROM users WHERE username = ? AND password = ?",
      [username, password]
    )) || [];
  return users.length > 0 ? users[0] : null;
};

const addUser = async (name: string, pw: string) => {
  await user_db.value?.execute(
    "INSERT INTO users (username, password) VALUES (?, ?)",
    [name, pw]
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

const getAllUsers = async () => {
  const users = await invoke("get_all_users");
  console.log(users);
};

const getUser = async () => {
  const user = await invoke("get_user");
  console.log(user);
  loginUsername.value = (user as User).username;
  loginPassword.value = (user as User).password;
};

const login = async (username: string, password: string) => {
  loginUser(username, password).then(async (success) => {
    if (success) {
      error.value = "";
      const login = { username, password, id: 1 };
      const user = await invoke("login", { user: login });
      console.log("Login successful", user);
      loggedInUser.value = user as User;
    } else {
      console.log("Login failed");
      error.value = "Invalid username or password";
    }
  });
};

const openFile = async () => {
  await invoke("open_file");
};

const saveFile = async () => {
  const data =
    (document.querySelector("#contents") as HTMLTextAreaElement)?.value || "";
  await invoke("save_file", { content: data });

  // clear the textarea after saving
  (document.querySelector("#contents") as HTMLTextAreaElement).value = "";
};

onMounted(() => {
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
});
</script>

<template>
  <div class="h-screen w-screen flex flex-col items-start justify-center p-5">
    <p class="mt-4 text-gray-600">demonstrating tauri functions</p>
    <div>
      <p v-if="loggedInUser" class="text-green-600">
        Logged in as: {{ loggedInUser.username }}
      </p>
      <p v-else class="text-red-600">Not logged in</p>
    </div>
    <div class="flex p-4 gap-4">
      <button @click="getAllUsers()">Get locale users</button>
      <button @click="getUser()">Get User</button>
    </div>
    <div class="flex gap-4">
      <input
        v-model="loginUsername"
        type="text"
        placeholder="Username"
        class="border-2 border-gray-300 rounded-md p-2 mt-4"
      />
      <input
        v-model="loginPassword"
        type="password"
        placeholder="Password"
        class="border-2 border-gray-300 rounded-md p-2 mt-4"
      />
      <button
        class="mt-4 submit-btn"
        @click="login(loginUsername, loginPassword)"
      >
        Login
      </button>
    </div>
    <p v-if="error" class="my-2 text-red-600">{{ error }}</p>
    <p class="mt-4 text-gray-600">demonstrating file dialogs</p>
    <div class="flex flex-row items-start gap-5">
      <textarea
        id="contents"
        class="w-full max-w-md h-32 border-2 border-gray-300 rounded-md p-2 mt-4"
        placeholder="File contents will appear here..."
      ></textarea>
      <div>
        <button class="mt-4 open-btn" @click="openFile()">Open File</button>
        <button class="mt-4 save-btn" @click="saveFile()">Save File</button>
      </div>
    </div>
    <div>
      <p class="mt-4 text-gray-600">
        demonstrating file dialogs SQLite database operations.
      </p>
      <div class="flex gap-4">
        <input
          v-model="username"
          type="text"
          placeholder="Username"
          class="border-2 border-gray-300 rounded-md p-2 mt-4"
        />
        <input
          v-model="password"
          type="password"
          placeholder="Password"
          class="border-2 border-gray-300 rounded-md p-2 mt-4"
        />
        <button class="mt-4 submit-btn" @click="addUser(username, password)">
          Add User
        </button>
        <button class="mt-4 clear-btn" @click="clearUsers()">
          Clear Users
        </button>
      </div>
      <div
        v-if="allUsers.length"
        class="flex flex-col mt-6 w-full max-w-md border-2 border-gray-300 rounded-md p-4 overflow-x-auto"
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
    </div>
  </div>
</template>

<style scoped></style>

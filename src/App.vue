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

const username = ref("");
const password = ref("");

const loadDatabase = async () => {
  const db = await Database.load("sqlite:mydatabase.db");
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

const queryUserById = async (id: number) => {
  const users: User[] =
    (await user_db.value?.select(
      "SELECT id, username, password FROM users WHERE id = ?",
      [id]
    )) || [];
  return users[0];
};

const loginUser = async (username: string, password: string) => {
  const users: User[] =
    (await user_db.value?.select(
      "SELECT id, username, password FROM users WHERE username = ? AND password = ?",
      [username, password]
    )) || [];
  return users.length > 0;
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
};

const login = async () => {
  const login = { username: "ras", password: "test", id: 1 };
  //await invoke("login", { user: login });
  await addUser(login.username, login.password);
};

const openFile = async () => {
  await invoke("open_file");
};

const saveFile = async () => {
  const data =
    (document.querySelector("#contents") as HTMLTextAreaElement)?.value || "";
  await invoke("save_file", { content: data });
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
    <div class="flex p-4 gap-4">
      <button @click="getAllUsers()">Get locale users</button>
      <button @click="login()">Login</button>
      <button @click="getUser()">Get User</button>
    </div>
    <p class="mt-4 text-gray-600">demonstrating file dialogs</p>
    <div class="flex p-4 gap-4">
      <button @click="openFile()">Open</button>
      <button @click="saveFile()">Save</button>
    </div>
    <textarea
      id="contents"
      class="border-2 border-gray-300 rounded-md p-2 w-96 h-48"
      placeholder="File content will appear here..."
    ></textarea>
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

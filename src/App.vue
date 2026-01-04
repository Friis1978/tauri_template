<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { onMounted } from "vue";

const getAllUsers = async () => {
  const users = await invoke("get_all_users");
  console.log(users);
};

const getUser = async () => {
  const user = await invoke("get_user");
  console.log(user);
};

const doLogin = async () => {
  const login = { username: "ras", password: "test", id: 1 };
  const result = await invoke("login", { user: login });
  console.log(result);
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
});
</script>

<template>
  <div class="h-screen w-screen flex flex-col items-center justify-center">
    <div class="flex p-4 gap-4">
      <button @click="getAllUsers()">Get All Users</button>
      <button @click="doLogin()">Login</button>
      <button @click="getUser()">Get User</button>
    </div>
    <div class="flex p-4 gap-4">
      <button @click="openFile()">Open</button>
      <button @click="saveFile()">Save</button>
    </div>
    <textarea
      id="contents"
      class="border-2 border-gray-300 rounded-md p-2 w-96 h-48"
      placeholder="File content will appear here..."
    ></textarea>
  </div>
</template>

<style scoped></style>

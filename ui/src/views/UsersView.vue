<template>
  <div>
    <div class="flex justify-between items-center mb-4">
      <div>
        <h2 class="text-xl font-bold">{{ t.users.title }}</h2>
        <p class="text-base-content/60 mt-1 text-sm">{{ t.users.description }}</p>
      </div>
      <div class="flex gap-2">
        <button class="btn btn-outline btn-xs" @click="showRoles = !showRoles">
          {{ showRoles ? t.users.showUsers : t.users.showRoles }}
        </button>
        <button class="btn btn-primary btn-xs" @click="openCreateModal">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5">
            <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
          </svg>
          {{ showRoles ? t.users.createRole : t.users.createUser }}
        </button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-md text-primary"></span>
    </div>

    <!-- Users View -->
    <div v-else-if="!showRoles">
      <!-- Empty state -->
      <div v-if="users.length === 0" class="text-center py-8">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16 mx-auto text-base-content/30 mb-3">
          <path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 0 0 2.625.372 9.337 9.337 0 0 0 4.121-.952 4.125 4.125 0 0 0-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 0 1 8.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0 1 11.964-3.07M12 6.375a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0Zm8.25 2.25a2.625 2.625 0 1 1-5.25 0 2.625 2.625 0 0 1 5.25 0Z" />
        </svg>
        <h3 class="text-lg font-semibold mb-2">{{ t.users.noUsers }}</h3>
        <p class="text-base-content/60 mb-4 text-sm">{{ t.users.noUsersDesc }}</p>
        <button class="btn btn-primary btn-xs" @click="openCreateModal">{{ t.users.createUser }}</button>
      </div>

      <!-- Users Table -->
      <div v-else class="overflow-x-auto bg-base-100 rounded-box shadow">
        <table class="table table-xs">
          <thead>
            <tr>
              <th class="text-xs">{{ t.users.username }}</th>
              <th class="text-xs">{{ t.users.email }}</th>
              <th class="text-xs">{{ t.users.role }}</th>
              <th class="text-xs">{{ t.users.status }}</th>
              <th class="text-xs">{{ t.users.created }}</th>
              <th class="text-xs">{{ t.users.actions }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="user in users" :key="user.id" class="hover">
              <td>
                <div class="font-semibold text-xs">{{ user.username }}</div>
              </td>
              <td>
                <div class="text-[10px]">{{ user.email || '-' }}</div>
              </td>
              <td>
                <div class="badge badge-ghost badge-xs">{{ user.role_name || t.users.noRole }}</div>
              </td>
              <td>
                <div :class="`badge badge-xs ${user.is_active ? 'badge-success' : 'badge-ghost'}`">
                  {{ user.is_active ? t.users.active : t.users.inactive }}
                </div>
              </td>
              <td class="text-[10px]">{{ formatDate(user.created_at) }}</td>
              <td>
                <div class="flex gap-1">
                  <button class="btn btn-xs btn-ghost" @click="editUser(user)">{{ t.users.edit }}</button>
                  <button class="btn btn-xs btn-ghost" @click="toggleUserStatus(user)">
                    {{ user.is_active ? t.users.deactivate : t.users.activate }}
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Roles View -->
    <div v-else>
      <!-- Empty state -->
      <div v-if="roles.length === 0" class="text-center py-8">
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-16 h-16 mx-auto text-base-content/30 mb-3">
          <path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75M21 12c0 1.268-.63 2.39-1.593 3.068a3.745 3.745 0 0 1-1.043 3.296 3.745 3.745 0 0 1-3.296 1.043A3.745 3.745 0 0 1 12 21c-1.268 0-2.39-.63-3.068-1.593a3.746 3.746 0 0 1-3.296-1.043 3.745 3.745 0 0 1-1.043-3.296A3.745 3.745 0 0 1 3 12c0-1.268.63-2.39 1.593-3.068a3.745 3.745 0 0 1 1.043-3.296 3.746 3.746 0 0 1 3.296-1.043A3.746 3.746 0 0 1 12 3c1.268 0 2.39.63 3.068 1.593a3.746 3.746 0 0 1 3.296 1.043 3.746 3.746 0 0 1 1.043 3.296A3.745 3.745 0 0 1 21 12Z" />
        </svg>
        <h3 class="text-lg font-semibold mb-2">{{ t.users.noRoles }}</h3>
        <p class="text-base-content/60 mb-4 text-sm">{{ t.users.noRolesDesc }}</p>
        <button class="btn btn-primary btn-xs" @click="openCreateModal">{{ t.users.createRole }}</button>
      </div>

      <!-- Roles Grid -->
      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div v-for="role in roles" :key="role.id" class="card bg-base-100 shadow">
          <div class="card-body p-3">
            <div class="flex items-center justify-between mb-2">
              <h3 class="card-title text-sm">{{ role.name }}</h3>
              <button class="btn btn-xs btn-ghost" @click="editRole(role)">{{ t.users.edit }}</button>
            </div>
            <p class="text-xs text-base-content/60 mb-3">{{ role.description || t.users.noDescription }}</p>
            <div class="flex flex-wrap gap-1">
              <span v-for="perm in role.permissions" :key="perm" class="badge badge-xs badge-outline">
                {{ perm }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Create/Edit User Modal -->
    <dialog ref="userModalRef" class="modal">
      <div class="modal-box p-4">
        <h3 class="font-bold text-base mb-3">{{ editingUser ? t.users.editUser : t.users.createUser }}</h3>
        <form @submit.prevent="handleUserSubmit">
          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.users.username }}</span>
            </label>
            <input
              v-model="userForm.username"
              type="text"
              class="input input-bordered input-sm"
              :required="!editingUser"
              :disabled="!!editingUser"
            />
          </div>
          <div class="form-control mb-3" v-if="!editingUser">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.users.password }}</span>
            </label>
            <input
              v-model="userForm.password"
              type="password"
              class="input input-bordered input-sm"
              :required="!editingUser"
            />
          </div>
          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.users.email }}</span>
            </label>
            <input v-model="userForm.email" type="email" class="input input-bordered input-sm" />
          </div>
          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.users.role }}</span>
            </label>
            <select v-model="userForm.role_id" class="select select-bordered select-sm">
              <option :value="undefined">{{ t.users.noRole }}</option>
              <option v-for="role in roles" :key="role.id" :value="role.id">{{ role.name }}</option>
            </select>
          </div>
          <div class="modal-action py-3">
            <button type="button" class="btn btn-xs" @click="closeUserModal">{{ t.users.cancel }}</button>
            <button type="submit" class="btn btn-primary btn-xs" :disabled="userSubmitting">
              <span v-if="userSubmitting" class="loading loading-spinner loading-xs"></span>
              {{ editingUser ? t.users.update : t.users.create }}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" class="modal-backdrop" @click="closeUserModal">
        <button>close</button>
      </form>
    </dialog>

    <!-- Create/Edit Role Modal -->
    <dialog ref="roleModalRef" class="modal">
      <div class="modal-box p-4">
        <h3 class="font-bold text-base mb-3">{{ editingRole ? t.users.editRole : t.users.createRole }}</h3>
        <form @submit.prevent="handleRoleSubmit">
          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.users.roleName }}</span>
            </label>
            <input v-model="roleForm.name" type="text" class="input input-bordered input-sm" required />
          </div>
          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.users.roleDescription }}</span>
            </label>
            <textarea v-model="roleForm.description" class="textarea textarea-bordered h-16 text-sm"></textarea>
          </div>
          <div class="form-control mb-3">
            <label class="label py-1">
              <span class="label-text font-semibold text-sm">{{ t.users.permissions }}</span>
            </label>
            <div class="border border-base-200 rounded-lg p-3 space-y-1.5">
              <label v-for="perm in availablePermissions" :key="perm" class="flex items-center gap-2">
                <input
                  type="checkbox"
                  :checked="roleForm.permissions.includes(perm)"
                  @change="togglePermission(perm)"
                  class="checkbox checkbox-xs"
                />
                <span class="text-xs">{{ perm }}</span>
              </label>
            </div>
          </div>
          <div class="modal-action py-3">
            <button type="button" class="btn btn-xs" @click="closeRoleModal">{{ t.users.cancel }}</button>
            <button type="submit" class="btn btn-primary btn-xs" :disabled="roleSubmitting">
              <span v-if="roleSubmitting" class="loading loading-spinner loading-xs"></span>
              {{ editingRole ? t.users.update : t.users.create }}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" class="modal-backdrop" @click="closeRoleModal">
        <button>close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { apiClient } from '@/api/client';
import { useLanguageStore } from '@/stores/language';
import { useToast } from '@/composables/useToast';
import type { UserResponse, RoleResponse } from '@/types/api';

const languageStore = useLanguageStore();
const t = computed(() => languageStore.t);
const { showError, showSuccess } = useToast();

const loading = ref(false);
const showRoles = ref(false);

const users = ref<UserResponse[]>([]);
const roles = ref<RoleResponse[]>([]);

const userModalRef = ref<HTMLDialogElement>();
const roleModalRef = ref<HTMLDialogElement>();

const editingUser = ref<UserResponse | null>(null);
const editingRole = ref<RoleResponse | null>(null);

const userSubmitting = ref(false);
const roleSubmitting = ref(false);

const userForm = reactive({
  username: '',
  password: '',
  email: '',
  role_id: undefined as number | undefined,
});

const roleForm = reactive({
  name: '',
  description: '',
  permissions: [] as string[],
});

const availablePermissions = [
  'cluster:read',
  'cluster:write',
  'cluster:delete',
  'topic:read',
  'topic:write',
  'topic:delete',
  'consumer_group:read',
  'consumer_group:write',
  'consumer_group:delete',
  'acl:read',
  'acl:write',
  'acl:delete',
  'quota:read',
  'quota:write',
  'user:read',
  'user:write',
  'admin',
];

async function fetchUsers() {
  users.value = await apiClient.getUsers();
}

async function fetchRoles() {
  roles.value = await apiClient.getRoles();
}

function openCreateModal() {
  if (showRoles.value) {
    roleForm.name = '';
    roleForm.description = '';
    roleForm.permissions = [];
    roleModalRef.value?.showModal();
  } else {
    editingUser.value = null;
    userForm.username = '';
    userForm.password = '';
    userForm.email = '';
    userForm.role_id = undefined;
    userModalRef.value?.showModal();
  }
}

function editUser(user: UserResponse) {
  editingUser.value = user;
  userForm.username = user.username;
  userForm.password = '';
  userForm.email = user.email || '';
  userForm.role_id = user.role_id;
  userModalRef.value?.showModal();
}

function editRole(role: RoleResponse) {
  editingRole.value = role;
  roleForm.name = role.name;
  roleForm.description = role.description || '';
  roleForm.permissions = [...role.permissions];
  roleModalRef.value?.showModal();
}

function closeUserModal() {
  userModalRef.value?.close();
  editingUser.value = null;
}

function closeRoleModal() {
  roleModalRef.value?.close();
  editingRole.value = null;
}

function togglePermission(perm: string) {
  const index = roleForm.permissions.indexOf(perm);
  if (index > -1) {
    roleForm.permissions.splice(index, 1);
  } else {
    roleForm.permissions.push(perm);
  }
}

async function handleUserSubmit() {
  userSubmitting.value = true;
  try {
    if (editingUser.value) {
      await apiClient.updateUser(editingUser.value.id, {
        email: userForm.email || undefined,
        role_id: userForm.role_id,
      });
    } else {
      await apiClient.createUser({
        username: userForm.username,
        password: userForm.password,
        email: userForm.email || undefined,
        role_id: userForm.role_id,
      });
    }
    showSuccess(t.value.users.updated);
    closeUserModal();
    fetchUsers();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    userSubmitting.value = false;
  }
}

async function handleRoleSubmit() {
  roleSubmitting.value = true;
  try {
    if (editingRole.value) {
      await apiClient.updateRole(editingRole.value.id, {
        name: roleForm.name,
        description: roleForm.description || undefined,
        permissions: roleForm.permissions,
      });
    } else {
      await apiClient.createRole({
        name: roleForm.name,
        description: roleForm.description || undefined,
        permissions: roleForm.permissions,
      });
    }
    showSuccess(t.value.users.updated);
    closeRoleModal();
    fetchRoles();
  } catch (e) {
    showError((e as { message: string }).message);
  } finally {
    roleSubmitting.value = false;
  }
}

async function toggleUserStatus(user: UserResponse) {
  try {
    await apiClient.updateUser(user.id, { is_active: !user.is_active });
    showSuccess(user.is_active ? t.value.users.deactivate : t.value.users.activate);
    fetchUsers();
  } catch (e) {
    showError((e as { message: string }).message);
  }
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

onMounted(() => {
  fetchUsers();
  fetchRoles();
});
</script>

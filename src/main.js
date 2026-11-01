import { invoke, open } from '@tauri-apps/api';

const $ = (id) => document.getElementById(id);

const localPath = $('localPath');
const uncPath = $('uncPath');
const credUser = $('credUser');
const credPass = $('credPass');
const chkSaveCred = $('chkSaveCred');
const btnBrowse = $('btnBrowse');
const btnMount = $('btnMount');
const btnUnmount = $('btnUnmount');
const status = $('status');

function show(msg, kind) {
  status.textContent = msg;
  status.className = kind === 'ok' ? 'ok' : 'err';
  status.style.display = msg ? 'block' : 'none';
  console.log(`[${kind}]`, msg);
}

function setButtons(isLinked) {
  btnMount.disabled = isLinked;
  btnUnmount.disabled = !isLinked;
  if (isLinked) {
    btnMount.textContent = '已挂载（可卸载）';
    btnMount.classList.remove('primary');
  } else {
    btnMount.textContent = '挂载 / 创建链接';
    btnMount.classList.add('primary');
  }
}

async function checkLink(lp) {
  if (!lp) {
    setButtons(false);
    show('', 'ok');
    return;
  }
  try {
    const target = await invoke('check_link', { localPath: lp });
    if (target) {
      setButtons(true);
      show(`当前已是符号链接，指向：${target}`, 'ok');
    } else {
      setButtons(false);
      show('', 'ok');
    }
  } catch (e) {
    setButtons(false);
    show('检查链接状态失败：' + String(e), 'err');
  }
}

async function pickFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: '',
    });
    if (selected === null || selected === '') return;
    const path = Array.isArray(selected) ? selected[0] : selected;
    localPath.value = path;
    await checkLink(path);
    show('', 'ok');
  } catch (e) {
    show('打开文件夹对话框失败：' + String(e), 'err');
  }
}

btnBrowse.addEventListener('click', pickFolder);

btnMount.addEventListener('click', async () => {
  const lp = localPath.value.trim();
  const un = uncPath.value.trim();
  if (!lp) return show('请输入本地文件夹路径', 'err');
  if (!un) return show('请输入远程 SMB 路径（UNC）', 'err');

  btnMount.disabled = true;
  show('正在挂载，请稍候……', 'ok');
  try {
    const result = await invoke('mount', {
      localPath: lp,
      uncPath: un,
      saveCreds: chkSaveCred.checked,
      username: credUser.value.trim() || null,
      password: credPass.value || null,
    });
    show('挂载成功：' + result, 'ok');
    await checkLink(lp);
  } catch (e) {
    show('挂载失败：' + String(e), 'err');
  } finally {
    btnMount.disabled = false;
  }
});

btnUnmount.addEventListener('click', async () => {
  const lp = localPath.value.trim();
  if (!lp) return show('请输入本地文件夹路径', 'err');

  btnUnmount.disabled = true;
  show('正在卸载，请稍候……', 'ok');
  try {
    const result = await invoke('unmount', { localPath: lp });
    show('卸载成功：' + result, 'ok');
    setButtons(false);
    show('', 'ok');
  } catch (e) {
    show('卸载失败：' + String(e), 'err');
  } finally {
    btnUnmount.disabled = false;
  }
});

// 输入变化时自动检查本地路径是否已是链接
let debounceTimer;
localPath.addEventListener('input', () => {
  show('', 'ok');
  clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    checkLink(localPath.value.trim());
  }, 300);
});

// 页面加载时检查当前本地路径状态
window.addEventListener('DOMContentLoaded', () => {
  checkLink(localPath.value.trim());
});

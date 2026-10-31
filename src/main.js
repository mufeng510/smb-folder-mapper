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
  } catch (e) {
    show('卸载失败：' + String(e), 'err');
  } finally {
    btnUnmount.disabled = false;
  }
});

// 输入变化时清空状态提示
[localPath, uncPath, credUser, credPass].forEach((el) => {
  el.addEventListener('input', () => show('', 'ok'));
});

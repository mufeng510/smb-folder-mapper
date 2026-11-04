import { invoke } from '@tauri-apps/api';

const $ = (id) => document.getElementById(id);

const localPath = $('localPath');
const uncPath = $('uncPath');
const credUser = $('credUser');
const credPass = $('credPass');
const chkSaveCred = $('chkSaveCred');
const btnBrowse = $('btnBrowse');
const btnMount = $('btnMount');
const btnUnmount = $('btnUnmount');
const btnRefreshList = $('btnRefreshList');
const linkList = $('linkList');
const linkListEmpty = $('linkListEmpty');
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

const checkLink = async (lp) => {
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
};

async function loadLinks() {
  try {
    const links = await invoke('get_links');
    renderLinks(links);
  } catch (e) {
    show('加载链接列表失败：' + String(e), 'err');
  }
}

function renderLinks(links) {
  linkList.innerHTML = '';
  if (!links || links.length === 0) {
    linkListEmpty.style.display = 'block';
    return;
  }
  linkListEmpty.style.display = 'none';
  for (const link of links) {
    const div = document.createElement('div');
    div.className = 'link-item';
    div.innerHTML = `
      <div class="local-path" title="${link.local_path}">${escapeHtml(link.local_path)}</div>
      <div class="unc-path" title="${link.unc_path}">${escapeHtml(link.unc_path)}</div>
      <button class="btn-remove" data-path="${escapeAttr(link.local_path)}">取消链接</button>
    `;
    const btn = div.querySelector('.btn-remove');
    btn.addEventListener('click', async () => {
      btn.disabled = true;
      btn.textContent = '处理中...';
      try {
        await invoke('remove_link', { localPath: link.local_path });
        show('已删除链接：' + link.local_path, 'ok');
        await loadLinks();
        // 如果当前输入框就是这个路径，重置按钮状态
        if (localPath.value.trim() === link.local_path) {
          setButtons(false);
          show('', 'ok');
        }
      } catch (e) {
        show('删除链接失败：' + String(e), 'err');
      } finally {
        btn.disabled = false;
        btn.textContent = '取消链接';
      }
    });
    linkList.appendChild(div);
  }
}

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function escapeAttr(text) {
  return text.replace(/"/g, '&quot;');
}

async function pickFolder() {
  try {
    const selected = await invoke('plugin:dialog|open', { options: { directory: true } });
    if (typeof selected === 'string') {
      localPath.value = selected;
      checkLink(selected);
      show('', 'ok');
    }
  } catch (e) {
    show('打开文件夹选择对话框失败：' + String(e), 'err');
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
    await loadLinks();
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
    await loadLinks();
  } catch (e) {
    show('卸载失败：' + String(e), 'err');
  } finally {
    btnUnmount.disabled = false;
  }
});

btnRefreshList.addEventListener('click', async () => {
  btnRefreshList.disabled = true;
  btnRefreshList.textContent = '刷新中...';
  try {
    await loadLinks();
    show('链接列表已刷新', 'ok');
  } catch (e) {
    show('刷新失败：' + String(e), 'err');
  } finally {
    btnRefreshList.disabled = false;
    btnRefreshList.textContent = '刷新';
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

// 页面加载时检查当前本地路径状态并加载链接列表
window.addEventListener('DOMContentLoaded', async () => {
  checkLink(localPath.value.trim());
  await loadLinks();
});

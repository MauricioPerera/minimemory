import { useState } from 'react';
import {
  Plus,
  Key,
  Copy,
  Check,
  Trash2,
  ToggleLeft,
  ToggleRight,
  Clock,
  Activity,
  Shield,
  ShieldOff,
  AlertCircle,
} from 'lucide-react';
import {
  useAgentTokens,
  useAgentTokenStats,
  useCreateAgentToken,
  useDeleteAgentToken,
  useToggleAgentToken,
} from '../api/hooks';
import type { AgentToken, AgentPermission } from '../api/types';

export function AgentTokens() {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

  const { data: tokensData, isLoading } = useAgentTokens();
  const { data: statsData } = useAgentTokenStats();
  const deleteMutation = useDeleteAgentToken();
  const toggleMutation = useToggleAgentToken();

  const copyToClipboard = async (id: string) => {
    await navigator.clipboard.writeText(id);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const handleDelete = (id: string) => {
    deleteMutation.mutate(id, {
      onSuccess: () => setDeleteConfirm(null),
    });
  };

  const handleToggle = (id: string) => {
    toggleMutation.mutate(id);
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  const isExpired = (token: AgentToken) => {
    return token.expiresAt ? token.expiresAt < Date.now() : false;
  };

  return (
    <div className="space-y-6 pb-16">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white">Agent Tokens</h2>
          <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">
            Manage access tokens for AI agents connecting via MCP
          </p>
        </div>
        <button
          onClick={() => setShowCreateModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
        >
          <Plus className="w-4 h-4" />
          Create Token
        </button>
      </div>

      {/* Stats Cards */}
      {statsData && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm">
            <div className="flex items-center gap-2 text-gray-500 dark:text-gray-400 text-sm">
              <Key className="w-4 h-4" />
              Total Tokens
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {statsData.stats.total}
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm">
            <div className="flex items-center gap-2 text-green-500 text-sm">
              <Shield className="w-4 h-4" />
              Active
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {statsData.stats.active}
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm">
            <div className="flex items-center gap-2 text-gray-400 text-sm">
              <ShieldOff className="w-4 h-4" />
              Inactive
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {statsData.stats.inactive}
            </div>
          </div>
          <div className="bg-white dark:bg-gray-800 rounded-xl p-4 shadow-sm">
            <div className="flex items-center gap-2 text-gray-500 dark:text-gray-400 text-sm">
              <Activity className="w-4 h-4" />
              Total Uses
            </div>
            <div className="text-2xl font-bold text-gray-900 dark:text-white mt-1">
              {statsData.stats.totalUseCount}
            </div>
          </div>
        </div>
      )}

      {/* Tokens List */}
      {isLoading ? (
        <div className="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm">
          <div className="animate-pulse text-gray-400">Loading tokens...</div>
        </div>
      ) : tokensData?.tokens.length === 0 ? (
        <div className="bg-white dark:bg-gray-800 rounded-xl p-12 text-center shadow-sm">
          <Key className="w-12 h-12 mx-auto text-gray-300 dark:text-gray-600 mb-4" />
          <div className="text-gray-500 dark:text-gray-400 text-lg mb-2">No agent tokens yet</div>
          <div className="text-gray-400 dark:text-gray-500 text-sm mb-4">
            Create a token to allow AI agents to access your memories via MCP
          </div>
          <button
            onClick={() => setShowCreateModal(true)}
            className="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors"
          >
            <Plus className="w-4 h-4" />
            Create Your First Token
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          {tokensData?.tokens.map((token) => (
            <div
              key={token.id}
              className={`bg-white dark:bg-gray-800 rounded-xl p-5 shadow-sm border-l-4 transition-colors ${
                !token.isActive || isExpired(token)
                  ? 'border-gray-300 dark:border-gray-600 opacity-60'
                  : 'border-indigo-500'
              }`}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3">
                    <h3 className="text-lg font-semibold text-gray-900 dark:text-white truncate">
                      {token.name}
                    </h3>
                    {!token.isActive && (
                      <span className="px-2 py-0.5 text-xs bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                        Inactive
                      </span>
                    )}
                    {isExpired(token) && (
                      <span className="px-2 py-0.5 text-xs bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400 rounded">
                        Expired
                      </span>
                    )}
                  </div>
                  {token.description && (
                    <p className="text-gray-500 dark:text-gray-400 text-sm mt-1">{token.description}</p>
                  )}

                  {/* Token ID */}
                  <div className="flex items-center gap-2 mt-3">
                    <code className="text-xs bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded font-mono text-gray-700 dark:text-gray-300">
                      {token.id}
                    </code>
                    <button
                      onClick={() => copyToClipboard(token.id)}
                      className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-colors"
                      title="Copy token ID"
                    >
                      {copiedId === token.id ? (
                        <Check className="w-4 h-4 text-green-500" />
                      ) : (
                        <Copy className="w-4 h-4 text-gray-400" />
                      )}
                    </button>
                  </div>

                  {/* Token Details */}
                  <div className="flex flex-wrap items-center gap-4 mt-3 text-sm">
                    <div className="flex items-center gap-1 text-gray-500 dark:text-gray-400">
                      <Shield className="w-4 h-4" />
                      <span>
                        {token.permissions.includes('read') && token.permissions.includes('write')
                          ? 'Read & Write'
                          : token.permissions.includes('read')
                          ? 'Read Only'
                          : 'Write Only'}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 text-gray-500 dark:text-gray-400">
                      <Activity className="w-4 h-4" />
                      <span>{token.useCount} uses</span>
                    </div>
                    {token.lastUsedAt && (
                      <div className="flex items-center gap-1 text-gray-500 dark:text-gray-400">
                        <Clock className="w-4 h-4" />
                        <span>Last used {formatDate(token.lastUsedAt)}</span>
                      </div>
                    )}
                    {token.expiresAt && (
                      <div
                        className={`flex items-center gap-1 ${
                          isExpired(token) ? 'text-red-500' : 'text-gray-500 dark:text-gray-400'
                        }`}
                      >
                        <AlertCircle className="w-4 h-4" />
                        <span>
                          {isExpired(token) ? 'Expired' : `Expires ${formatDate(token.expiresAt)}`}
                        </span>
                      </div>
                    )}
                  </div>

                  {/* Allowed Memories */}
                  <div className="mt-3">
                    <span className="text-xs text-gray-500 dark:text-gray-400">Allowed memories: </span>
                    <span className="text-xs text-gray-700 dark:text-gray-300">
                      {token.allowedMemories.includes('*')
                        ? 'All memories'
                        : `${token.allowedMemories.length} specific memories`}
                    </span>
                  </div>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => handleToggle(token.id)}
                    disabled={toggleMutation.isPending}
                    className={`p-2 rounded-lg transition-colors ${
                      token.isActive
                        ? 'hover:bg-gray-100 dark:hover:bg-gray-700 text-green-500'
                        : 'hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-400'
                    }`}
                    title={token.isActive ? 'Deactivate token' : 'Activate token'}
                  >
                    {token.isActive ? (
                      <ToggleRight className="w-6 h-6" />
                    ) : (
                      <ToggleLeft className="w-6 h-6" />
                    )}
                  </button>
                  {deleteConfirm === token.id ? (
                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => handleDelete(token.id)}
                        disabled={deleteMutation.isPending}
                        className="px-3 py-1 bg-red-600 text-white text-sm rounded hover:bg-red-700 transition-colors"
                      >
                        {deleteMutation.isPending ? 'Deleting...' : 'Confirm'}
                      </button>
                      <button
                        onClick={() => setDeleteConfirm(null)}
                        className="px-3 py-1 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 text-sm rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors"
                      >
                        Cancel
                      </button>
                    </div>
                  ) : (
                    <button
                      onClick={() => setDeleteConfirm(token.id)}
                      className="p-2 hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500 rounded-lg transition-colors"
                      title="Delete token"
                    >
                      <Trash2 className="w-5 h-5" />
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Create Token Modal */}
      <CreateTokenModal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
      />
    </div>
  );
}

// Create Token Modal Component
function CreateTokenModal({
  isOpen,
  onClose,
}: {
  isOpen: boolean;
  onClose: () => void;
}) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [permissions, setPermissions] = useState<AgentPermission[]>(['read', 'write']);
  const [allowAll, setAllowAll] = useState(true);
  const [expiresIn, setExpiresIn] = useState<string>('never');

  const createMutation = useCreateAgentToken();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    let expiresAt: number | undefined;
    if (expiresIn !== 'never') {
      const days = parseInt(expiresIn, 10);
      expiresAt = Date.now() + days * 24 * 60 * 60 * 1000;
    }

    createMutation.mutate(
      {
        name,
        description: description || undefined,
        permissions,
        allowedMemories: allowAll ? ['*'] : [],
        expiresAt,
      },
      {
        onSuccess: () => {
          onClose();
          setName('');
          setDescription('');
          setPermissions(['read', 'write']);
          setAllowAll(true);
          setExpiresIn('never');
        },
      }
    );
  };

  const togglePermission = (perm: AgentPermission) => {
    if (permissions.includes(perm)) {
      if (permissions.length > 1) {
        setPermissions(permissions.filter((p) => p !== perm));
      }
    } else {
      setPermissions([...permissions, perm]);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />
      <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full mx-4 p-6">
        <h3 className="text-xl font-bold text-gray-900 dark:text-white mb-4">Create Agent Token</h3>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Name */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Token Name *
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., Work Assistant"
              required
              maxLength={100}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
            />
          </div>

          {/* Description */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Description
            </label>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="e.g., Token for work-related memories"
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
            />
          </div>

          {/* Permissions */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Permissions
            </label>
            <div className="flex gap-3">
              <button
                type="button"
                onClick={() => togglePermission('read')}
                className={`flex-1 py-2 px-3 rounded-lg border text-sm font-medium transition-colors ${
                  permissions.includes('read')
                    ? 'bg-indigo-100 dark:bg-indigo-900/30 border-indigo-500 text-indigo-700 dark:text-indigo-300'
                    : 'bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'
                }`}
              >
                Read
              </button>
              <button
                type="button"
                onClick={() => togglePermission('write')}
                className={`flex-1 py-2 px-3 rounded-lg border text-sm font-medium transition-colors ${
                  permissions.includes('write')
                    ? 'bg-indigo-100 dark:bg-indigo-900/30 border-indigo-500 text-indigo-700 dark:text-indigo-300'
                    : 'bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'
                }`}
              >
                Write
              </button>
            </div>
          </div>

          {/* Memory Access */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Memory Access
            </label>
            <div className="flex gap-3">
              <button
                type="button"
                onClick={() => setAllowAll(true)}
                className={`flex-1 py-2 px-3 rounded-lg border text-sm font-medium transition-colors ${
                  allowAll
                    ? 'bg-indigo-100 dark:bg-indigo-900/30 border-indigo-500 text-indigo-700 dark:text-indigo-300'
                    : 'bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'
                }`}
              >
                All Memories
              </button>
              <button
                type="button"
                onClick={() => setAllowAll(false)}
                className={`flex-1 py-2 px-3 rounded-lg border text-sm font-medium transition-colors ${
                  !allowAll
                    ? 'bg-indigo-100 dark:bg-indigo-900/30 border-indigo-500 text-indigo-700 dark:text-indigo-300'
                    : 'bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300'
                }`}
              >
                Specific Only
              </button>
            </div>
            {!allowAll && (
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-2">
                You can add specific memory IDs after creating the token.
              </p>
            )}
          </div>

          {/* Expiration */}
          <div>
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Expiration
            </label>
            <select
              value={expiresIn}
              onChange={(e) => setExpiresIn(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
            >
              <option value="never">Never expires</option>
              <option value="7">7 days</option>
              <option value="30">30 days</option>
              <option value="90">90 days</option>
              <option value="365">1 year</option>
            </select>
          </div>

          {/* Error */}
          {createMutation.isError && (
            <div className="p-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 rounded-lg text-sm">
              {createMutation.error.message}
            </div>
          )}

          {/* Actions */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 py-2 px-4 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!name || createMutation.isPending}
              className="flex-1 py-2 px-4 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {createMutation.isPending ? 'Creating...' : 'Create Token'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

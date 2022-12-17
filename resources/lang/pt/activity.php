<?php

/**
 * Contains all of the translation strings for different activity log
 * events. These should be keyed by the value in front of the colon (:)
 * in the event name. If there is no colon present, they should live at
 * the top level.
 */
return [
    'auth' => [
        'fail' => 'falhou ao carregar',
        'success' => 'Logado a',
        'password-reset' => 'Redefinição de senha',
        'reset-password' => 'Redefinição de senha solicitada',
        'checkpoint' => 'Autenticação de dois fatores solicitada',
        'recovery-token' => 'Token de recuperação de dois fatores usado',
        'token' => 'Desafio de dois fatores resolvido',
        'ip-blocked' => 'Solicitação bloqueada de endereço IP não listado para :identifier',
        'sftp' => [
            'fail' => 'Login sftp com falha',
        ],
    ],
    'user' => [
        'account' => [
            'email-changed' => 'E-mail alterado de :old para :new',
            'password-changed' => 'senha alterada',
        ],
        'api-key' => [
            'create' => 'Criada nova API key :identifier',
            'delete' => 'Deletada a API key :identifier',
        ],
        'ssh-key' => [
            'create' => 'Adicionada SSH key :fingerprint to account',
            'delete' => 'Removida SSH key :fingerprint from account',
        ],
        'two-factor' => [
            'create' => 'Auth de dois fatores habilitado',
            'delete' => 'Auth de dois fatores desabilitado',
        ],
    ],
    'server' => [
        'reinstall' => 'Servidor reinstalado',
        'console' => [
            'command' => 'Excutado ":command" no servidor',
        ],
        'power' => [
            'start' => 'Iniciado o servidor',
            'stop' => 'Parou o servidor',
            'restart' => 'Reiniciou o servidor',
            'kill' => 'Matou o processo do servidor',
        ],
        'backup' => [
            'download' => 'Baixou o backup :name ',
            'delete' => 'Deletou o backup :name ',
            'restore' => 'Restaurou o backup :name (deleted files: :truncate)',
            'restore-complete' => 'Restauração concluída do :name backup',
            'restore-failed' => 'Falhou em completar a restauração do :name backup',
            'start' => 'Iniciou um novo backup :name',
            'complete' => 'Marcou que o :name backup foi completo',
            'fail' => 'Marcou que o :name backup falhou',
            'lock' => 'Trancou o :name backup',
            'unlock' => 'Destrancou o :name backup',
        ],
        'database' => [
            'create' => 'Criou novo banco de dados :name',
            'rotate-password' => 'Senha girada para banco de dados :name',
            'delete' => 'Banco de dados excluído :name',
        ],
        'file' => [
            'compress_one' => 'Comprimido :directory:file',
            'compress_other' => 'Comprimido :count arquivos em :directory',
            'read' => 'Visualizado o conteúdo de :file',
            'copy' => 'Criou uma cópia de :file',
            'create-directory' => 'Diretório criado :directory:name',
            'decompress' => 'Descompactado :files no :directory',
            'delete_one' => 'Deletado :directory:files.0',
            'delete_other' => 'Deletado :count arquivos em :directory',
            'download' => 'Baixado :file',
            'pull' => 'Baixado um arquivo remoto de :url para :directory',
            'rename_one' => 'Renomeado :directory:files.0.de para :directory:files.0.to',
            'rename_other' => 'Renomeado :count arquivos em :directory',
            'write' => 'Escreveu novo conteúdo para :file',
            'upload' => 'Começou um upload de arquivo',
            'uploaded' => 'Carregado :directory:file',
        ],
        'sftp' => [
            'denied' => 'Acesso SFTP bloqueado devido a permissões',
            'create_one' => 'Criado :files.0',
            'create_other' => 'Criado :count new files',
            'write_one' => 'Modificado o conteúdo de :files.0',
            'write_other' => 'Modificado o conteúdo de :count files',
            'delete_one' => 'Deletado :files.0',
            'delete_other' => 'Deletado :count files',
            'create-directory_one' => 'Criado o :files.0 directory',
            'create-directory_other' => 'Criado :count Diretórios',
            'rename_one' => 'Renomeado :files.0.de para :files.0.to',
            'rename_other' => 'Renomeado ou movido :count arquivos',
        ],
        'allocation' => [
            'create' => 'Adicionado :allocation para o servidor',
            'notes' => 'Atualizado as notas para :allocation de ":old" para ":new"',
            'primary' => 'Pôr :allocation como a alocação do servidor principal',
            'delete' => 'Deletou a :allocation alocação',
        ],
        'schedule' => [
            'create' => 'Criou o :name schedule',
            'update' => 'Atualizou o :name schedule',
            'execute' => 'Executou manualmente o :name schedule',
            'delete' => 'Deletou o :name schedule',
        ],
        'task' => [
            'create' => 'Criou um novo ":action" tarefa para o :name schedule',
            'update' => 'Atualizou o ":action" tarefa para o :name schedule',
            'delete' => 'Excluiu uma tarefa para o :name schedule',
        ],
        'settings' => [
            'rename' => 'Renomeaou o servidor de :old para :new',
        ],
        'startup' => [
            'edit' => 'Mudou o :variable variável de ":old" para ":new"',
            'image' => 'Atualizou a Imagem Docker para o servidor de :old para :new',
        ],
        'subuser' => [
            'create' => 'Adicionou o :email como subusuário',
            'update' => 'Atualizou as permissões do subusuário :email',
            'delete' => 'Removeu :email como subusuário',
        ],
    ],
];

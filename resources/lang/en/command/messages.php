<?php

return [
    'location' => [
        'no_location_found' => 'Não foi possível localizar um registro que corresponde ao código curto fornecido.',
        'ask_short' => 'Código curto de localização',
        'ask_long' => 'Descrição da localização',
        'created' => 'Criou com sucesso um novo local (:name) com uma ID de :id.',
        'deleted' => 'Excluiu com sucesso o local solicitado.',
    ],
    'user' => [
        'search_users' => 'Digite um nome de usuário, ID de usuário ou endereço de e-mail',
        'select_search_user' => 'ID do usuário para excluir (Enter \'0\' to re-search)',
        'deleted' => 'Usuário excluído com sucesso do Painel.',
        'confirm_delete' => 'Tem certeza de que deseja excluir este usuário do Painel?',
        'no_users_found' => 'Não foram encontrados usuários para o termo de pesquisa fornecido.',
        'multiple_found' => 'Várias contas foram encontradas para o usuário fornecido, incapaz de excluir um usuário por causa do --no-interaction flag.',
        'ask_admin' => 'Este usuário é um administrador?',
        'ask_email' => 'Endereço de E-mail',
        'ask_username' => 'Nome de Usuario',
        'ask_name_first' => 'Primeiro Nome',
        'ask_name_last' => 'Ultimo Nome',
        'ask_password' => 'Senha',
        'ask_password_tip' => 'If você gostaria de criar uma conta com uma senha aleatória enviada pelo usuário, re-executar este comando (CTRL+C) e passar o `--no-password` flag.',
        'ask_password_help' => 'As senhas devem ter pelo menos 8 caracteres de comprimento e conter pelo menos uma letra maiúscula e número.',
        '2fa_help_text' => [
            'Este comando desativará a autenticação de 2 fatores para um user\'s conta se ele está ativado. Isso só deve ser usado como um comando de recuperação de conta se o usuário estiver bloqueado fora de sua conta.',
            'Se não é isso que você queria fazer, pressione CTRL+C para sair desse processo.',
        ],
        '2fa_disabled' => 'A autenticação de 2 fatores foi desativada para :email.',
    ],
    'schedule' => [
        'output_line' => 'Expedição de trabalho para a primeira tarefa em `:schedule` (:hash).',
    ],
    'maintenance' => [
        'deleting_service_backup' => 'Excluindo arquivo de backup do serviço :file.',
    ],
    'server' => [
        'rebuild_failed' => 'Pedido de reconstrução para ":name" (#:id) no Node ":node" falhou com erro: :message',
        'reinstall' => [
            'failed' => 'Solicitação de reinstalação para ":name" (#:id) no Node ":node" falhou com erro: :message',
            'confirm' => 'Você está prestes a reinstalar contra um grupo de servidores. Você deseja continuar?',
        ],
        'power' => [
            'confirm' => 'Você está prestes a realizar um :action contra :count servidores. Você deseja continuar?',
            'action_failed' => 'Pedido de ação de energia para ":name" (#:id) no Node ":node" falhou com o errr: :message',
        ],
    ],
    'environment' => [
        'mail' => [
            'ask_smtp_host' => 'SMTP Host (e.g. smtp.gmail.com)',
            'ask_smtp_port' => 'SMTP Porta',
            'ask_smtp_username' => 'SMTP Usuario',
            'ask_smtp_password' => 'SMTP Senha',
            'ask_mailgun_domain' => 'Mailgun Dominio',
            'ask_mailgun_endpoint' => 'Mailgun Extremidade',
            'ask_mailgun_secret' => 'Mailgun Secreto',
            'ask_mandrill_secret' => 'Mandrill Secreto',
            'ask_postmark_username' => 'Chave de API de marca posta',
            'ask_driver' => 'Qual driver deve ser usado para enviar e-mails?',
            'ask_mail_from' => 'Os e-mails de endereço de e-mail devem ser originários de',
            'ask_mail_name' => 'Nome que os e-mails devem aparecer de',
            'ask_encryption' => 'Método de criptografia para usar',
        ],
    ],
];

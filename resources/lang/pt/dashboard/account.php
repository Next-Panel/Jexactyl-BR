<?php

return [
    'email' => [
        'title' => 'Atualize seu E-mail',
        'updated' => 'Seu endereço de E-mail foi atualizado.',
    ],
    'password' => [
        'title' => 'Altere sua senha',
        'requirements' => 'Sua nova senha deve ter pelo menos 8 caracteres de comprimento.',
        'updated' => 'Sua senha foi atualizada.',
    ],
    'two_factor' => [
        'button' => 'Configure autenticação de 2 fatores',
        'disabled' => 'A autenticação de dois fatores foi desativada em sua conta. Você não será mais solicitado a fornecer um token ao fazer login.',
        'enabled' => 'A autenticação de dois fatores foi ativada em sua conta! A partir de agora, ao fazer login, você será obrigado a fornecer o código gerado pelo seu dispositivo.',
        'invalid' => 'O token fornecido era inválido.',
        'setup' => [
            'title' => 'Configuração de autenticação de dois fatores',
            'help' => 'Can\'t Digitalizar o código? Digite o código abaixo em seu aplicativo:',
            'field' => 'Coloque o token',
        ],
        'disable' => [
            'title' => 'Desativar a autenticação de dois fatores',
            'field' => 'Coloque o token',
        ],
    ],
];

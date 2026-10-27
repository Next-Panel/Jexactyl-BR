<?php

// Tradução pt-BR das mensagens administrativas de Nests.
// Nests, Eggs, daemons e variáveis mantêm a terminologia técnica do Jexactyl.

return [
    'notices' => [
        'created' => 'Um novo Nest, :name, foi criado com sucesso.',
        'deleted' => 'O Nest solicitado foi excluído do Painel com sucesso.',
        'updated' => 'As opções de configuração do Nest foram atualizadas com sucesso.',
    ],
    'eggs' => [
        'notices' => [
            'imported' => 'Este Egg e as variáveis associadas a ele foram importados com sucesso.',
            'updated_via_import' => 'Este Egg foi atualizado usando o arquivo fornecido.',
            'deleted' => 'O Egg solicitado foi excluído do Painel com sucesso.',
            'updated' => 'A configuração do Egg foi atualizada com sucesso.',
            'script_updated' => 'O script de instalação do Egg foi atualizado e será executado sempre que servidores forem instalados.',
            'egg_created' => 'Um novo Egg foi criado com sucesso. Você precisará reiniciar quaisquer daemons em execução para aplicar este novo Egg.',
        ],
    ],
    'variables' => [
        'notices' => [
            'variable_deleted' => 'A variável ":variable" foi excluída e não estará mais disponível para os servidores depois que forem reconstruídos.',
            'variable_updated' => 'A variável ":variable" foi atualizada. Você precisará reconstruir os servidores que usam essa variável para aplicar as alterações.',
            'variable_created' => 'A nova variável foi criada com sucesso e atribuída a este Egg.',
        ],
    ],
];

<?php
/**
 * Pterodactyl - Panel
 * Copyright (c) 2015 - 2017 Dane Everitt <dane@daneeveritt.com>.
 *
 * This software is licensed under the terms of the MIT license.
 * https://opensource.org/licenses/MIT
 */

return [
    'notices' => [
        'created' => 'Um novo ninho, :name, foi criado com sucesso.',
        'deleted' => 'Excluiu com sucesso o ninho solicitado do Painel.',
        'updated' => 'Atualizou com sucesso as opções de configuração do ninho.',
    ],
    'eggs' => [
        'notices' => [
            'imported' => 'Importou este egg com sucesso e suas variáveis associadas.',
            'updated_via_import' => 'Este egg foi atualizado usando o arquivo fornecido.',
            'deleted' => 'Excluiu com sucesso o egg solicitado do Painel.',
            'updated' => 'A configuração do egg foi atualizada com sucesso.',
            'script_updated' => 'O script de instalação de eggs foi atualizado e será executado sempre que os servidores forem instalados.',
            'egg_created' => 'Um novo egg foi colocado com sucesso. Você precisará reiniciar quaisquer daemons em execução para aplicar este novo egg.',
        ],
    ],
    'variables' => [
        'notices' => [
            'variable_deleted' => 'A variável ":variable" foi excluído e não estará mais disponível para servidores uma vez reconstruídos.',
            'variable_updated' => 'A variável ":variable" foi atualizado. Você precisará reconstruir quaisquer servidores usando essa variável para aplicar alterações.',
            'variable_created' => 'Nova variável foi criada com sucesso e atribuída a este Egg.',
        ],
    ],
];

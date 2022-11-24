@extends('layouts.admin')

@section('title')
    Nests &rarr; Egg: {{ $egg->name }}
@endsection

@section('content-header')
    <h1>{{ $egg->name }}<small>{{ str_limit($egg->description, 50) }}</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li><a href="{{ route('admin.nests') }}">Nests</a></li>
        <li><a href="{{ route('admin.nests.view', $egg->nest->id) }}">{{ $egg->nest->name }}</a></li>
        <li class="active">{{ $egg->name }}</li>
    </ol>
@endsection

@section('content')
<div class="row">
    <div class="col-xs-12">
        <div class="nav-tabs-custom nav-tabs-floating">
            <ul class="nav nav-tabs">
                <li class="active"><a href="{{ route('admin.nests.egg.view', $egg->id) }}">Configurações</a></li>
                <li><a href="{{ route('admin.nests.egg.variables', $egg->id) }}">Variáveis</a></li>
                <li><a href="{{ route('admin.nests.egg.scripts', $egg->id) }}">Instalação do Script</a></li>
            </ul>
        </div>
    </div>
</div>
<form action="{{ route('admin.nests.egg.view', $egg->id) }}" enctype="multipart/form-data" method="POST">
    <div class="row">
        <div class="col-xs-12">
            <div class="box box-danger">
                <div class="box-body">
                    <div class="row">
                        <div class="col-xs-8">
                            <div class="form-group no-margin-bottom">
                                <label for="pName" class="control-label">Arquivo do Egg</label>
                                <div>
                                    <input type="file" name="import_file" class="form-control" style="border: 0;margin-left:-10px;" />
                                    <p class="text-muted small no-margin-bottom"> Se você gostaria de substituir as configurações para este Egg carregando um novo arquivo JSON, basta selecioná-lo aqui e pressionar " Atualizar Egg ". Isso não alterará nenhuma cadeia de caracteres de inicialização ou imagens do Docker existentes para servidores existentes.</p>
                                </div>
                            </div>
                        </div>
                        <div class="col-xs-4">
                            {!! csrf_field() !!}
                            <button type="submit" name="_method" value="PUT" class="btn btn-sm btn-danger pull-right">Atualizar Egg</button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</form>
<form action="{{ route('admin.nests.egg.view', $egg->id) }}" method="POST">
    <div class="row">
        <div class="col-xs-12">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Configurações</h3>
                </div>
                <div class="box-body">
                    <div class="row">
                        <div class="col-sm-6">
                            <div class="form-group">
                                <label for="pName" class="control-label">Nome <span class="field-required"></span></label>
                                <input type="text" id="pName" name="name" value="{{ $egg->name }}" class="form-control" />
                                <p class="text-muted small">Um nome simples e legível por humanos para usar como identificador para este Egg.</p>
                            </div>
                            <div class="form-group">
                                <label for="pUuid" class="control-label">UUID</label>
                                <input type="text" id="pUuid" readonly value="{{ $egg->uuid }}" class="form-control" />
                                <p class="text-muted small">Este é o identificador globalmente exclusivo para este Egg que o Daemon usa como um identificador.</p>
                            </div>
                            <div class="form-group">
                                <label for="pAuthor" class="control-label">Autor</label>
                                <input type="text" id="pAuthor" readonly value="{{ $egg->author }}" class="form-control" />
                                <p class="text-muted small">O autor desta versão do Egg. Carregar uma nova configuração do Egg de um autor diferente mudará isso.</p>
                            </div>
                            <div class="form-group">
                                <label for="pDockerImage" class="control-label">Imagem Docker <span class="field-required"></span></label>
                                <textarea id="pDockerImages" name="docker_images" class="form-control" rows="4">{{ implode(PHP_EOL, $images) }}</textarea>
                                <p class="text-muted small">
                                    As imagens do docker disponíveis para os servidores que usam esse egg. Digite um por linha.
                                    Os usuários poderão selecionar a partir desta lista de imagens se mais de um valor for fornecido.
                                    Opcionalmente, um nome de exibição pode ser fornecido prefixando a imagem com o nome
                                    seguido de um caractere de canalização, e depois a URL da imagem. Exemplo: <code>Nome de exibição|ghcr.io/my/egg</code>
                                </p>
                            </div>
                            <div class="form-group">
                                <div class="checkbox checkbox-primary no-margin-bottom">
                                    <input id="pForceOutgoingIp" name="force_outgoing_ip" type="checkbox" value="1" @if($egg->force_outgoing_ip) checked @endif />
                                    <label for="pForceOutgoingIp" class="strong">Forçar o IP de saída</label>
                                    <p class="text-muted small">
                                        Força todo o tráfego de saída da rede a ter seu IP de origem NAT no IP do IP primário de alocação do servidor.
                                        Necessário para que certos jogos funcionem corretamente quando o Node tem múltiplos endereços IP públicos.
                                        <br>
                                        <strong>
                                            A habilitação desta opção desabilitará a rede interna para qualquer servidor que utilize este egg,
                                            fazendo com que eles não possam acessar internamente outros servidores no mesmo node.
                                        </strong>
                                    </p>
                                </div>
                            </div>

                        </div>
                        <div class="col-sm-6">
                            <div class="form-group">
                                <label for="pDescription" class="control-label">Descrição</label>
                                <textarea id="pDescription" name="description" class="form-control" rows="8">{{ $egg->description }}</textarea>
                                <p class="text-muted small">Uma descrição deste Egg que será exibido em todo o Painel, conforme necessário.</p>
                            </div>
                            <div class="form-group">
                                <label for="pStartup" class="control-label">Comando de inicialização <span class="field-required"></span></label>
                                <textarea id="pStartup" name="startup" class="form-control" rows="8">{{ $egg->startup }}</textarea>
                                <p class="text-muted small">O comando de inicialização padrão que deve ser usado para novos servidores usando este Egg.</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div class="col-xs-12">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Gestão de Processos</h3>
                </div>
                <div class="box-body">
                    <div class="row">
                        <div class="col-xs-12">
                            <div class="alert alert-warning">
                                <p>As opções de configuração a seguir não devem ser editadas, a menos que você entenda como esse sistema funciona. Se modificado incorretamente, é possível que o daemon quebre.</p>
                                <p>Todos os campos são obrigatórios, a menos que você selecione uma opção separada na lista suspensa 'Copiar configurações de', caso em que os campos podem ser deixados em branco para usar os valores desse egg.</p>
                            </div>
                        </div>
                        <div class="col-sm-6">
                            <div class="form-group">
                                <label for="pConfigFrom" class="form-label">Copiar configurações de</label>
                                <select name="config_from" id="pConfigFrom" class="form-control">
                                    <option value="">Nada</option>
                                    @foreach($egg->nest->eggs as $o)
                                        <option value="{{ $o->id }}" {{ ($egg->config_from !== $o->id) ?: 'selected' }}>{{ $o->name }} &lt;{{ $o->author }}&gt;</option>
                                    @endforeach
                                </select>
                                <p class="text-muted small">Se você quiser usar as configurações padrão de outro Egg, selecione-o no menu acima.</p>
                            </div>
                            <div class="form-group">
                                <label for="pConfigStop" class="form-label">Comando de Parar</label>
                                <input type="text" id="pConfigStop" name="config_stop" class="form-control" value="{{ $egg->config_stop }}" />
                                <p class="text-muted small">O comando que deve ser enviado aos processos do servidor para interrompê-los normalmente. Se você precisar enviar um <code>SIGINT</code> você deve digitar <code>^C</code> aqui.</p>
                            </div>
                            <div class="form-group">
                                <label for="pConfigLogs" class="form-label">Configuração de log</label>
                                <textarea data-action="handle-tabs" id="pConfigLogs" name="config_logs" class="form-control" rows="6">{{ ! is_null($egg->config_logs) ? json_encode(json_decode($egg->config_logs), JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES) : '' }}</textarea>
                                <p class="text-muted small">Essa deve ser uma representação JSON de onde os arquivos de log estão armazenados e se o daemon deve ou não criar logs personalizados.</p>
                            </div>
                        </div>
                        <div class="col-sm-6">
                            <div class="form-group">
                                <label for="pConfigFiles" class="form-label">Arquivos de configuração</label>
                                <textarea data-action="handle-tabs" id="pConfigFiles" name="config_files" class="form-control" rows="6">{{ ! is_null($egg->config_files) ? json_encode(json_decode($egg->config_files), JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES) : '' }}</textarea>
                                <p class="text-muted small">Esta deve ser uma representação JSON dos arquivos de configuração a serem modificados e que partes devem ser alteradas.</p>
                            </div>
                            <div class="form-group">
                                <label for="pConfigStartup" class="form-label">Configuração de inicialização</label>
                                <textarea data-action="handle-tabs" id="pConfigStartup" name="config_startup" class="form-control" rows="6">{{ ! is_null($egg->config_startup) ? json_encode(json_decode($egg->config_startup), JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES) : '' }}</textarea>
                                <p class="text-muted small">Esta deve ser uma representação JSON dos valores que o daemon deve estar procurando ao iniciar um servidor para determinar a conclusão.</p>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box-footer">
                    {!! csrf_field() !!}
                    <button type="submit" name="_method" value="PATCH" class="btn btn-primary btn-sm pull-right">Salvar</button>
                    <a href="{{ route('admin.nests.egg.export', $egg->id) }}" class="btn btn-sm btn-info pull-right" style="margin-right:10px;">Exportar</a>
                    <button id="deleteButton" type="submit" name="_method" value="DELETE" class="btn btn-danger btn-sm muted muted-hover">
                        <i class="fa fa-trash-o"></i>
                    </button>
                </div>
            </div>
        </div>
    </div>
</form>
@endsection

@section('footer-scripts')
    @parent
    <script>
    $('#pConfigFrom').select2();
    $('#deleteButton').on('mouseenter', function (event) {
        $(this).find('i').html(' Delete Egg');
    }).on('mouseleave', function (event) {
        $(this).find('i').html('');
    });
    $('textarea[data-action="handle-tabs"]').on('keydown', function(event) {
        if (event.keyCode === 9) {
            event.preventDefault();

            var curPos = $(this)[0].selectionStart;
            var prepend = $(this).val().substr(0, curPos);
            var append = $(this).val().substr(curPos);

            $(this).val(prepend + '    ' + append);
        }
    });
    </script>
@endsection

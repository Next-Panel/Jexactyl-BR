@extends('layouts.admin')

@section('title')
    Nests &rarr; Novo Egg
@endsection

@section('content-header')
    <h1>Novo Egg<small>Criar um novo egg para atribuir aos servidores.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li><a href="{{ route('admin.nests') }}">Nests</a></li>
        <li class="active">Novo Egg</li>
    </ol>
@endsection

@section('content')
<form action="{{ route('admin.nests.egg.new') }}" method="POST">
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
                                <label for="pNestId" class="form-label">Nest Associado</label>
                                <div>
                                    <select name="nest_id" id="pNestId">
                                        @foreach($nests as $nest)
                                            <option value="{{ $nest->id }}" {{ old('nest_id') != $nest->id ?: 'selected' }}>{{ $nest->name }} &lt;{{ $nest->author }}&gt;</option>
                                        @endforeach
                                    </select>
                                    <p class="text-muted small">Pense em um Nest como uma categoria. Você pode colocar vários eggs em um nest, mas considere colocar apenas eggs que estejam relacionados entre si em cada nest.</p>
                                </div>
                            </div>
                            <div class="form-group">
                                <label for="pName" class="form-label">Nome</label>
                                <input type="text" id="pName" name="name" value="{{ old('name') }}" class="form-control" />
                                <p class="text-muted small">Um nome simples, legível pelo homem, para ser usado como identificador deste Eggs. Isto é o que os usuários verão como seu tipo de servidor de jogo.</p>
                            </div>
                            <div class="form-group">
                                <label for="pDescription" class="form-label">Descrição</label>
                                <textarea id="pDescription" name="description" class="form-control" rows="8">{{ old('description') }}</textarea>
                                <p class="text-muted small">Uma descrição deste egg.</p>
                            </div>
                            <div class="form-group">
                                <div class="checkbox checkbox-primary no-margin-bottom">
                                    <input id="pForceOutgoingIp" name="force_outgoing_ip" type="checkbox" value="1" {{ \Pterodactyl\Helpers\Utilities::checked('force_outgoing_ip', 0) }} />
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
                                <label for="pDockerImage" class="control-label">Imagem do Docker</label>
                                <textarea id="pDockerImages" name="docker_images" rows="4" placeholder="quay.io/pterodactyl/service" class="form-control">{{ old('docker_images') }}</textarea>
                                <p class="text-muted small">As imagens dos dockers disponíveis para os servidores que utilizam este egg. Insira uma por linha. Os usuários poderão selecionar a partir desta lista de imagens se mais de um valor for fornecido.</p>
                            </div>
                            <div class="form-group">
                                <label for="pStartup" class="control-label">Comando de inicialização</label>
                                <textarea id="pStartup" name="startup" class="form-control" rows="10">{{ old('startup') }}</textarea>
                                <p class="text-muted small">O comando de inicialização padrão que deve ser usado para novos servidores criados com este Egg. Você pode alterar este por servidor conforme necessário.</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        <div class="col-xs-12">
            <div class="box">
                <div class="box-header with-border">
                    <h3 class="box-title">Gestão de processos</h3>
                </div>
                <div class="box-body">
                    <div class="row">
                        <div class="col-xs-12">
                            <div class="alert alert-warning">
                                <p>Todos os campos são obrigatórios a menos que você selecione uma opção separada da caixa suspensa "Copiar Configurações de", caso em que os campos podem ser deixados em branco para usar os valores dessa opção.</p>
                            </div>
                        </div>
                        <div class="col-sm-6">
                            <div class="form-group">
                                <label for="pConfigFrom" class="form-label">Configurações de cópia de</label>
                                <select name="config_from" id="pConfigFrom" class="form-control">
                                    <option value="">Nada</option>
                                </select>
                                <p class="text-muted small">Se você gostaria de definir as configurações padrão a partir de outro Egg, selecione-o a partir do menu suspenso acima.</p>
                            </div>
                            <div class="form-group">
                                <label for="pConfigStop" class="form-label">Comando de Parar</label>
                                <input type="text" id="pConfigStop" name="config_stop" class="form-control" value="{{ old('config_stop') }}" />
                                <p class="text-muted small">O comando que deve ser enviado aos processos do servidor para interrompê-los normalmente. Se você precisar enviar um <code>SIGINT</code> você deve digitar <code>^C</code> aqui.</p>
                            </div>
                            <div class="form-group">
                                <label for="pConfigLogs" class="form-label">Configuração de log</label>
                                <textarea data-action="handle-tabs" id="pConfigLogs" name="config_logs" class="form-control" rows="6">{{ old('config_logs') }}</textarea>
                                <p class="text-muted small">Essa deve ser uma representação JSON de onde os arquivos de log são armazenados e se o daemon deve ou não estar criando logs personalizados.</p>
                            </div>
                        </div>
                        <div class="col-sm-6">
                            <div class="form-group">
                                <label for="pConfigFiles" class="form-label">Arquivos de configuração</label>
                                <textarea data-action="handle-tabs" id="pConfigFiles" name="config_files" class="form-control" rows="6">{{ old('config_files') }}</textarea>
                                <p class="text-muted small">Esta deve ser uma representação JSON dos arquivos de configuração a serem modificados e que partes devem ser alteradas.</p>
                            </div>
                            <div class="form-group">
                                <label for="pConfigStartup" class="form-label">Configuração do Iniciar</label>
                                <textarea data-action="handle-tabs" id="pConfigStartup" name="config_startup" class="form-control" rows="6">{{ old('config_startup') }}</textarea>
                                <p class="text-muted small">Esta deve ser uma representação JSON dos valores que o daemon deve estar procurando ao iniciar um servidor para determinar a conclusão.</p>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="box-footer">
                    {!! csrf_field() !!}
                    <button type="submit" class="btn btn-success btn-sm pull-right">Criar</button>
                </div>
            </div>
        </div>
    </div>
</form>
@endsection

@section('footer-scripts')
    @parent
    {!! Theme::js('vendor/lodash/lodash.js') !!}
    <script>
    $(document).ready(function() {
        $('#pNestId').select2().change();
        $('#pConfigFrom').select2();
    });
    $('#pNestId').on('change', function (event) {
        $('#pConfigFrom').html('<option value="">None</option>').select2({
            data: $.map(_.get(Pterodactyl.nests, $(this).val() + '.eggs', []), function (item) {
                return {
                    id: item.id,
                    text: item.name + ' <' + item.author + '>',
                };
            }),
        });
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
